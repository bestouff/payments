use compute::Accounts;
use read::read_transactions;
use write::write_accounts;

mod compute;
mod data;
mod read;
mod write;

fn main() -> Result<(), anyhow::Error> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        anyhow::bail!("usage: {} transactions.csv > accounts.csv", args[0]);
    }
    let mut accounts = Accounts::new();
    read_transactions(std::fs::File::open(&args[1])?, &mut accounts)?;
    write_accounts(std::io::stdout(), &accounts)?;
    Ok(())
}
