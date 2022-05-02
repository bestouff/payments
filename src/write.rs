use crate::compute::Accounts;

/// Basic CSV exporter for `Accounts`
pub(crate) fn write_accounts<W: std::io::Write>(
    writer: W,
    accounts: &Accounts,
) -> Result<(), anyhow::Error> {
    let mut wtr = csv::Writer::from_writer(writer);
    for account in accounts.accounts.values() {
        wtr.serialize(account)?;
    }
    wtr.flush()?;
    Ok(())
}
