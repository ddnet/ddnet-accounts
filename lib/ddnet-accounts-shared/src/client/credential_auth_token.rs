use serde::{Deserialize, Serialize};
use strum::{EnumString, IntoStaticStr};

/// The operation for what this authorized credential token should do.
#[derive(Debug, Serialize, Deserialize, IntoStaticStr, EnumString, Clone, Copy, PartialEq, Eq)]
#[strum(serialize_all = "lowercase")]
pub enum CredentialAuthTokenOperation {
    /// Login using these credentials.
    Login,
    /// Link the credential to an account
    /// (e.g. email or steam).
    LinkCredential,
    /// Unlink the credential from its account
    /// (e.g. email or steam).
    /// If the credential is the last bound to
    /// the account this operation will fail and
    /// [`super::account_token::AccountTokenOperation::Delete`]
    /// should be used instead.
    UnlinkCredential,
}

/// A secret key used for a verification process.
pub type SecretKey = [u8; 32];

/// A request for a token that is used for the
/// email credential operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialAuthTokenEmailRequest {
    /// The email of the account to log into.
    pub email: email_address::EmailAddress,
    /// A secret key that was generated through
    /// a verification process (e.g. captchas).
    /// It is optional, since these verification
    /// processes differ from user to user.
    pub secret_key: Option<SecretKey>,
    /// The operation that this credential authorization
    /// should perform.
    pub op: CredentialAuthTokenOperation,
}

/// A request for a token that is used for the
/// steam credential operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialAuthTokenSteamRequest {
    /// The session token generated on the steam client
    /// for the account to log into.
    pub steam_ticket: Vec<u8>,
    /// A secret key that was generated through
    /// a verification process (e.g. captchas).
    /// It is optional, since these verification
    /// processes differ from user to user.
    pub secret_key: Option<SecretKey>,
    /// The operation that this credential authorization
    /// should perform.
    pub op: CredentialAuthTokenOperation,
}
