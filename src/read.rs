use crate::data::{Error, Transaction, SIGNIFICANT_DIGITS};

/// Trait for doing something with a `Transaction` read from a CSV file
/// (or received from elsewhere). Used by the main business logic to apply
/// operations on `Accounts`, but also used for mock tests to check we get the
/// correct results from reading a CSV stream.
pub(crate) trait TransactionUser {
    fn use_tx(&mut self, tx: Transaction) -> Result<(), Error>;
}

/// Simple CSV importer for `Transaction`s.
pub(crate) fn read_transactions<R: std::io::Read, U: TransactionUser>(
    reader: R,
    user: &mut U,
) -> Result<(), anyhow::Error> {
    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_reader(reader);
    for result in rdr.deserialize() {
        let mut tx: Transaction = result?;
        if let Some(mut amount) = tx.amount {
            amount.rescale(SIGNIFICANT_DIGITS);
            tx.amount = Some(amount);
        }
        if let Err(e) = user.use_tx(tx) {
            // Really crude error handling, we'd want something a bit more sophisticated IRL
            eprintln!("Transaction {} failed: {e}", tx.id);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        data::{Transaction, TxType::*},
        read::{read_transactions, TransactionUser},
    };
    use rust_decimal_macros::dec;

    #[test]
    fn read_tx() {
        #[derive(Default)]
        struct TxStorage {
            txst: Vec<Transaction>,
        }
        impl TransactionUser for TxStorage {
            fn use_tx(&mut self, tx: crate::data::Transaction) -> Result<(), crate::data::Error> {
                Ok(self.txst.push(tx))
            }
        }
        let mut storage = TxStorage::default();
        let transactions_csv = b"\
type,       client, tx, amount
deposit,    1,      1,  1.0
deposit,    2,      2,  2.0
deposit,    1,      3,  2.0
withdrawal, 1,      4,  1.5
withdrawal, 2,      5,  3.0
dispute,    1,      3,
";
        read_transactions(&transactions_csv[..], &mut storage).unwrap();
        assert_eq!(
            storage.txst,
            [
                Transaction {
                    txtype: Deposit,
                    client: 1,
                    id: 1,
                    amount: Some(dec!(1.0))
                },
                Transaction {
                    txtype: Deposit,
                    client: 2,
                    id: 2,
                    amount: Some(dec!(2.0))
                },
                Transaction {
                    txtype: Deposit,
                    client: 1,
                    id: 3,
                    amount: Some(dec!(2.0))
                },
                Transaction {
                    txtype: Withdrawal,
                    client: 1,
                    id: 4,
                    amount: Some(dec!(1.5))
                },
                Transaction {
                    txtype: Withdrawal,
                    client: 2,
                    id: 5,
                    amount: Some(dec!(3.0))
                },
                Transaction {
                    txtype: Dispute,
                    client: 1,
                    id: 3,
                    amount: None
                },
            ]
        )
    }
}
