use serde::{Deserialize, Serialize};
use strum::{EnumString, IntoStaticStr};

use crate::account_server::otp::Otp;

/// A token previously sent to email or generated
/// for a steam account, that can be used to perform various
/// actions on an account, e.g. deleting it or removing/revoking
/// all active sessions.
pub type AccountToken = Otp;

/// The operation for what this account token was generated for.
#[derive(Debug, Serialize, Deserialize, IntoStaticStr, EnumString, Clone, Copy, PartialEq, Eq)]
#[strum(serialize_all = "lowercase")]
pub enum AccountTokenOperation {
    /// Logout all sessions at once.
    LogoutAll,
    /// Link another credential to this account
    /// (e.g. email or steam).
    LinkCredential,
    /// Delete the account.
    Delete,
}

/// A secret key used for a verification process.
pub type SecretKey = [u8; 32];

/// A request for an account token by email.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountTokenEmailRequest {
    /// The email of the account.
    pub email: email_address::EmailAddress,
    /// The operation this account token should validate.
    pub op: AccountTokenOperation,
    /// A secret key that was generated through
    /// a verification process (e.g. captchas).
    /// It is optional, since these verification
    /// processes differ from user to user.
    pub secret_key: Option<SecretKey>,
}

/// A request for an account token by steam.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountTokenSteamRequest {
    /// The ticket from steam to verify the steamid for
    /// the account.
    pub steam_ticket: Vec<u8>,
    /// The operation this account token should validate.
    pub op: AccountTokenOperation,
    /// A secret key that was generated through
    /// a verification process (e.g. captchas).
    /// It is optional, since these verification
    /// processes differ from user to user.
    pub secret_key: Option<SecretKey>,
}
