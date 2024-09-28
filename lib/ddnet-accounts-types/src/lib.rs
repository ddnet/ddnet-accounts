//! This crate contains custom types used for the account system.
//! It should generally not depend on crates that cannot be compiled
//! to all rust targets (e.g. WASM).

#![deny(missing_docs)]
#![deny(warnings)]
#![deny(clippy::nursery)]
#![deny(clippy::all)]

/// Types related to an account on the account server
pub mod account_id;
