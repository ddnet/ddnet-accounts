//! This crate contains everything that is
//! required to do account related operations.
//! That includes all operations on the server
//! aswell as on the client, aswell as the account
//! server itself.
//! This crate is not intended for creating UI,
//! any game logic or the communication.

#![deny(missing_docs)]
#![deny(warnings)]
#![deny(clippy::nursery)]
#![deny(clippy::all)]

/// Everything account related on the account server
pub mod account_server;
/// Everything related to creating certificates
pub mod cert;
/// Everything account related for clients
pub mod client;
/// Everything account related for the game server
pub mod game_server;
