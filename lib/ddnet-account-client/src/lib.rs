//! This crate contains a base implementation for
//! a client to do account related operations.
//! It helps sending data, storing results persistently.
//! This crate is not intended for creating UI,
//! any game logic nor knowing about the communication details
//! (be it UDP, HTTP or other stuff).
//! It uses interfaces to abstract such concepts away.

#![deny(missing_docs)]
#![deny(warnings)]
#![deny(clippy::nursery)]
#![deny(clippy::all)]

pub(crate) mod safe_interface;

/// Requests the account info of the account.
pub mod account_info;
/// Requests an account token email based.
pub mod account_token;
/// Operations related to getting the account server certificates
pub mod certs;
/// Requests a token for an email based login.
pub mod credential_auth_token;
/// Requests to delete the account.
pub mod delete;
/// Types related to errors during client operations.
pub mod errors;
/// Communication interface for the client to
/// do requests to the account server.
pub mod interface;
/// Requests to link another credential to an
/// existing account.
pub mod link_credential;
/// Requests to create a new login for the corresponding
/// account.
pub mod login;
/// Request to log out the current user session.
pub mod logout;
/// Request to log out all sessions of a user.
pub mod logout_all;
/// Sign an already existing session key-pair
/// with a certificate on the account server.
pub mod sign;
/// Requests to unlink a credential from an account.
pub mod unlink_credential;
