# Toy Payments System

This is my solution to a toy payments system exercise.

## Completeness

All cases should have been implemented, as far as I could understand the specs.  
If some cases (be them edge cases or error cases) aren't handled it's because I forgot them or
didn't read the spec properly.

## Correctness

There are cases where I don't use the type system to ensure correctness but I probably should have:

- the currency amounts are store in a `Decimal` type which can go negative, so some tests have been
  introduced to check for that; the type system could have ensured this invariant (but wouldn't
  totally eliminate the tests).
- the transactions are stored more-or-less as deserialized; they should probably be stored as an
  `enum` type where e.g. only the `Deposit` and `Withdrawal` variants contain an `amount` of
  currency.

Also:

- I didn't totally understand how the mechanism for chargebacks is supposed to work, maybe my
  English is lacking; I implemented what I got from the specs.
- the spec isn't clear if zero-amount transactions are allowed, so I allowed them.
- I didn't see a mean to unfreeze an account from the spec, so on my code once it's
  frozen an account is basically dead.

But otherwise the code passes `cargo run -- transactions.csv`, `cargo test` & `cargo clippy`.

## Safety and Robustness

I'm a Rust programmer, I avoid `unsafe` code like plague, my code should be robust.  
Errors are handled via a custom `Error` type for business errors (invalid transactions) and
the usual error mechanism for I/O errors (thanks to `anyhow`).

## Efficiency

Transactions are streamed from the input file; only transactions with their own id are stored
(`Deposit` and `Withdrawal`).

If the code was bundled in a server and `Accounts::use_tx()` was to be called multithreaded,
I'd probably type `Accounts::accounts` as `RwLock<HashMap<ClientId, Mutex<Account>>>` so if
the account exists I can access it mutably while having a shared access to the collection,
and if not I'll have a mutable access to the collection and (after checking again) I can
create the account in it.

## Maintainability

The code is parcimoniously spread with comments and should be clear enough to understand
and modify. But who am I to judge ?
