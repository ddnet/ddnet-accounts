use ddnet_accounts_types::account_id::AccountId;
use serde::{Deserialize, Serialize};

/// A linked credential type of an account
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CredentialType {
    /// The partial readable email as string
    Email(String),
    /// The steam id
    Steam(i64),
}

/// The response of an account info request from the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountInfoResponse {
    /// The account id of the account
    pub account_id: AccountId,
    /// The UTC creation date of the account
    pub creation_date: chrono::DateTime<chrono::Utc>,
    /// the credentials linked to this account
    pub credentials: Vec<CredentialType>,
}
