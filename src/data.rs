use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type ClientId = u16;
pub type TxId = u32;

pub const SIGNIFICANT_DIGITS: u32 = 4;

/// This is our `Account` structure we work with. You'll note it has no `total` field
/// because it's a kind of "virtual" field whose value is always `available + held`. So
/// instead of manually maintaining an invariant everywhere, we'll just compute it at the only
/// time we need it: at serialization time.
/// See `AccountSerializer` for details
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
#[serde(into = "AccountSerializer")]
pub(crate) struct Account {
    pub client: ClientId,
    pub available: Decimal,
    pub held: Decimal,
    pub locked: bool,
}

/// This is our proxy for serializing `Account`: it will compute its
/// "virtual field" `total` just before serialization.
#[derive(Serialize)]
pub(crate) struct AccountSerializer {
    pub client: ClientId,
    pub available: Decimal,
    pub held: Decimal,
    pub total: Decimal,
    pub locked: bool,
}

impl From<Account> for AccountSerializer {
    fn from(account: Account) -> Self {
        Self {
            client: account.client,
            total: account.available + account.held,
            available: account.available,
            held: account.held,
            locked: account.locked,
        }
    }
}

/// Store for a transaction; note that the `amount` field can't be negative - this isn't explicit
/// in the specs but makes sense, so it's enforced in the code. Also the spec isn't clear if
/// zero amounts are allowed, so they are indeed allowed (even if that makes little sense, it
/// does not seem like an impossible transaction).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub(crate) struct Transaction {
    #[serde(rename = "type")]
    pub txtype: TxType,
    pub client: ClientId,
    #[serde(rename = "tx")]
    pub id: TxId,
    pub amount: Option<Decimal>,
}

/// Different types of transaction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum TxType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

/// Transaction error handling; these are just here to show how it's done and are
/// incomplete for a real life use. For example, `InsufficientFunds` probably should tell us
/// which transaction tried to withdraw the funds, and from which client account it is.
#[derive(Error, Debug, PartialEq, Eq)]
pub enum Error {
    #[error("Duplicate transaction #{0}")]
    DuplicateTransaction(TxId),
    #[error("Transaction #{0} not found")]
    TransactionNotFound(TxId),
    #[error("Insufficient funds for operation (asked {asked} while {available} available)")]
    InsufficientFunds { asked: Decimal, available: Decimal },
    #[error("Account already locked")]
    AccountLocked,
    #[error("Transaction amount must be positive")]
    NegativeAmount,
    #[error("Transaction amount is missing for dispute/withdrawal")]
    MissingAmount,
    #[error("Transaction amount shouldn't be there for dispute/resolve/chargeback")]
    UnattendedforAmount,
    #[error("Only deposits can be disputed/resolved/chargedback")]
    WrongDispute,
    #[error("Attempt to dispute/resolve/chargeback on a different client account")]
    DisputeMismatch,
}
