use crate::{
    data::{Account, ClientId, Error, Transaction, TxId, TxType::*},
    read::TransactionUser,
};
use std::collections::HashMap;

/// This is where accounts are store; they are created on the fly when reading the
/// transactions. The exercise was single-threaded so no protections for MT.
#[derive(Debug)]
pub(crate) struct Accounts {
    pub accounts: HashMap<ClientId, Account>,
    txset: HashMap<TxId, Transaction>,
}

impl Accounts {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            txset: HashMap::new(),
        }
    }
}

/// This is where the business logic stands. Maybe I could have factorized some
/// repeated idioms into their own little function, like:
/// ```rust
/// if self.txset.insert(tx.id, tx).is_some() {
///     return Err(Error::DuplicateTransaction(tx.id));
/// }
/// ```
/// into something like `self.add_tx(tx)?;` but I'm not even sure the added code would
/// have made the boilerplate any more clear. YMMV.
impl TransactionUser for Accounts {
    fn use_tx(&mut self, tx: Transaction) -> Result<(), Error> {
        if tx.amount.unwrap_or_default().is_sign_negative() {
            return Err(Error::NegativeAmount);
        }
        let account = self.accounts.entry(tx.client).or_insert(Account {
            client: tx.client,
            ..Account::default()
        });
        // FIXME: the spec doesn't say when an account is to be unlocked,
        // that means that after a chargeback an account is basically dead.
        // If some operation is able to unlock the account, this test should
        // move in the appropriate operations.
        if account.locked {
            return Err(Error::AccountLocked);
        }
        match tx.txtype {
            Deposit => {
                if self.txset.insert(tx.id, tx).is_some() {
                    return Err(Error::DuplicateTransaction(tx.id));
                }
                let amount = tx.amount.ok_or(Error::MissingAmount)?;
                account.available += amount;
            }
            Withdrawal => {
                if self.txset.insert(tx.id, tx).is_some() {
                    return Err(Error::DuplicateTransaction(tx.id));
                }
                let amount = tx.amount.ok_or(Error::MissingAmount)?;
                if account.available < amount {
                    return Err(Error::InsufficientFunds {
                        asked: amount,
                        available: account.available,
                    });
                }
                account.available -= amount;
            }
            Dispute => {
                if tx.amount.is_some() {
                    return Err(Error::UnattendedforAmount);
                }
                let tx = self
                    .txset
                    .get(&tx.id)
                    .ok_or(Error::TransactionNotFound(tx.id))?;
                if tx.txtype != Deposit {
                    return Err(Error::WrongDispute);
                }
                if tx.client != account.client {
                    return Err(Error::DisputeMismatch);
                }
                let amount = tx.amount.ok_or(Error::MissingAmount)?;
                if account.available < amount {
                    return Err(Error::InsufficientFunds {
                        asked: amount,
                        available: account.available,
                    });
                }
                account.available -= amount;
                account.held += amount;
            }
            Resolve => {
                if tx.amount.is_some() {
                    return Err(Error::UnattendedforAmount);
                }
                let tx = self
                    .txset
                    .get(&tx.id)
                    .ok_or(Error::TransactionNotFound(tx.id))?;
                if tx.txtype != Deposit {
                    return Err(Error::WrongDispute);
                }
                if tx.client != account.client {
                    return Err(Error::DisputeMismatch);
                }
                let amount = tx.amount.ok_or(Error::MissingAmount)?;
                if account.held < amount {
                    return Err(Error::InsufficientFunds {
                        asked: amount,
                        available: account.held,
                    });
                }
                account.available += amount;
                account.held -= amount;
            }
            Chargeback => {
                if tx.amount.is_some() {
                    return Err(Error::UnattendedforAmount);
                }
                let tx = self
                    .txset
                    .get(&tx.id)
                    .ok_or(Error::TransactionNotFound(tx.id))?;
                if tx.txtype != Deposit {
                    return Err(Error::WrongDispute);
                }
                if tx.client != account.client {
                    return Err(Error::DisputeMismatch);
                }
                let amount = tx.amount.ok_or(Error::MissingAmount)?;
                if account.held < amount {
                    return Err(Error::InsufficientFunds {
                        asked: amount,
                        available: account.held,
                    });
                }
                account.held -= amount;
                account.locked = true;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        data::{Account, Error, Transaction, TxType::*},
        read::TransactionUser,
    };
    use rust_decimal_macros::dec;

    use super::Accounts;

    #[test]
    fn test_deposit() {
        let mut accounts = Accounts::new();
        accounts
            .use_tx(Transaction {
                txtype: Deposit,
                client: 5,
                id: 1,
                amount: Some(dec!(100)),
            })
            .unwrap();
        assert_eq!(
            accounts.accounts[&5],
            Account {
                client: 5,
                available: dec!(100),
                held: dec!(0),
                locked: false,
            },
        );
    }
    #[test]
    fn test_withdrawal() {
        let mut accounts = Accounts::new();
        accounts
            .use_tx(Transaction {
                txtype: Deposit,
                client: 5,
                id: 1,
                amount: Some(dec!(100)),
            })
            .unwrap();
        accounts
            .use_tx(Transaction {
                txtype: Withdrawal,
                client: 5,
                id: 2,
                amount: Some(dec!(60)),
            })
            .unwrap();
        assert_eq!(
            accounts.accounts[&5],
            Account {
                client: 5,
                available: dec!(40),
                held: dec!(0),
                locked: false,
            },
        );
    }
    #[test]
    fn test_dispute() {
        let mut accounts = Accounts::new();
        accounts
            .use_tx(Transaction {
                txtype: Deposit,
                client: 5,
                id: 1,
                amount: Some(dec!(100)),
            })
            .unwrap();
        accounts
            .use_tx(Transaction {
                txtype: Dispute,
                client: 5,
                id: 1,
                amount: None,
            })
            .unwrap();
        assert_eq!(
            accounts.accounts[&5],
            Account {
                client: 5,
                available: dec!(0),
                held: dec!(100),
                locked: false,
            },
        );
    }
    #[test]
    fn test_resolve() {
        let mut accounts = Accounts::new();
        accounts
            .use_tx(Transaction {
                txtype: Deposit,
                client: 5,
                id: 1,
                amount: Some(dec!(100)),
            })
            .unwrap();
        accounts
            .use_tx(Transaction {
                txtype: Dispute,
                client: 5,
                id: 1,
                amount: None,
            })
            .unwrap();
        accounts
            .use_tx(Transaction {
                txtype: Resolve,
                client: 5,
                id: 1,
                amount: None,
            })
            .unwrap();
        assert_eq!(
            accounts.accounts[&5],
            Account {
                client: 5,
                available: dec!(100),
                held: dec!(0),
                locked: false,
            },
        );
    }
    #[test]
    fn test_chargeback() {
        let mut accounts = Accounts::new();
        accounts
            .use_tx(Transaction {
                txtype: Deposit,
                client: 5,
                id: 1,
                amount: Some(dec!(100)),
            })
            .unwrap();
        accounts
            .use_tx(Transaction {
                txtype: Dispute,
                client: 5,
                id: 1,
                amount: None,
            })
            .unwrap();
        accounts
            .use_tx(Transaction {
                txtype: Chargeback,
                client: 5,
                id: 1,
                amount: None,
            })
            .unwrap();
        assert_eq!(
            accounts.accounts[&5],
            Account {
                client: 5,
                available: dec!(0),
                held: dec!(0),
                locked: true,
            },
        );
    }
    #[test]
    fn test_withdrawal_insufficient_funds() {
        let mut accounts = Accounts::new();
        accounts
            .use_tx(Transaction {
                txtype: Deposit,
                client: 5,
                id: 1,
                amount: Some(dec!(100)),
            })
            .unwrap();
        assert_eq!(
            accounts.use_tx(Transaction {
                txtype: Withdrawal,
                client: 5,
                id: 2,
                amount: Some(dec!(200)),
            }),
            Err(Error::InsufficientFunds {
                asked: dec!(200),
                available: dec!(100)
            })
        );
    }
    #[test]
    fn test_locked_account() {
        let mut accounts = Accounts::new();
        accounts
            .use_tx(Transaction {
                txtype: Deposit,
                client: 5,
                id: 1,
                amount: Some(dec!(100)),
            })
            .unwrap();
        accounts.accounts.get_mut(&5).unwrap().locked = true;
        assert_eq!(
            accounts.use_tx(Transaction {
                txtype: Withdrawal,
                client: 5,
                id: 2,
                amount: Some(dec!(200)),
            }),
            Err(Error::AccountLocked)
        );
    }
    #[test]
    fn test_duplicate_transaction() {
        let mut accounts = Accounts::new();
        accounts
            .use_tx(Transaction {
                txtype: Deposit,
                client: 5,
                id: 1,
                amount: Some(dec!(100)),
            })
            .unwrap();
        assert_eq!(
            accounts.use_tx(Transaction {
                txtype: Withdrawal,
                client: 5,
                id: 1,
                amount: Some(dec!(200)),
            }),
            Err(Error::DuplicateTransaction(1))
        );
    }
    #[test]
    fn test_negative_amount() {
        let mut accounts = Accounts::new();
        assert_eq!(
            accounts.use_tx(Transaction {
                txtype: Deposit,
                client: 5,
                id: 1,
                amount: Some(dec!(-100)),
            }),
            Err(Error::NegativeAmount)
        );
        assert_eq!(
            accounts.use_tx(Transaction {
                txtype: Withdrawal,
                client: 5,
                id: 2,
                amount: Some(dec!(-100)),
            }),
            Err(Error::NegativeAmount)
        );
    }
    #[test]
    fn test_missing_amount() {
        let mut accounts = Accounts::new();
        assert_eq!(
            accounts.use_tx(Transaction {
                txtype: Deposit,
                client: 5,
                id: 1,
                amount: None,
            }),
            Err(Error::MissingAmount)
        );
    }
    #[test]
    fn test_unattended_amount() {
        let mut accounts = Accounts::new();
        accounts
            .use_tx(Transaction {
                txtype: Deposit,
                client: 5,
                id: 1,
                amount: Some(dec!(100)),
            })
            .unwrap();
        assert_eq!(
            accounts.use_tx(Transaction {
                txtype: Dispute,
                client: 5,
                id: 1,
                amount: Some(dec!(100)),
            }),
            Err(Error::UnattendedforAmount)
        );
    }
    #[test]
    fn test_transaction_not_found() {
        let mut accounts = Accounts::new();
        accounts
            .use_tx(Transaction {
                txtype: Deposit,
                client: 5,
                id: 1,
                amount: Some(dec!(100)),
            })
            .unwrap();
        assert_eq!(
            accounts.use_tx(Transaction {
                txtype: Dispute,
                client: 5,
                id: 2,
                amount: None,
            }),
            Err(Error::TransactionNotFound(2))
        );
    }
    #[test]
    fn test_dispute_mismatch() {
        let mut accounts = Accounts::new();
        accounts
            .use_tx(Transaction {
                txtype: Deposit,
                client: 5,
                id: 1,
                amount: Some(dec!(100)),
            })
            .unwrap();
        assert_eq!(
            accounts.use_tx(Transaction {
                txtype: Dispute,
                client: 2,
                id: 1,
                amount: None,
            }),
            Err(Error::DisputeMismatch)
        );
    }
    #[test]
    fn test_wrong_dispute() {
        let mut accounts = Accounts::new();
        accounts
            .use_tx(Transaction {
                txtype: Deposit,
                client: 5,
                id: 1,
                amount: Some(dec!(100)),
            })
            .unwrap();
        accounts
            .use_tx(Transaction {
                txtype: Withdrawal,
                client: 5,
                id: 2,
                amount: Some(dec!(60)),
            })
            .unwrap();
        assert_eq!(
            accounts.use_tx(Transaction {
                txtype: Dispute,
                client: 2,
                id: 2,
                amount: None,
            }),
            Err(Error::WrongDispute)
        );
    }
}
