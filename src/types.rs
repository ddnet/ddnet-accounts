use ddnet_accounts_shared::client::{
    account_token::AccountTokenOperation, credential_auth_token::CredentialAuthTokenOperation,
};
use serde::{Deserialize, Serialize};
use strum::{EnumString, IntoStaticStr};

// IMPORTANT: keep this in sync with the ty enum in src/setup/mysql/credential_auth_tokens.sql
/// The type of token that was created.
#[derive(Debug, Serialize, Deserialize, IntoStaticStr, EnumString, Clone, Copy)]
#[strum(serialize_all = "lowercase")]
pub enum TokenType {
    Email,
    Steam,
}

// IMPORTANT: keep this in sync with the ty enum in src/setup/mysql/account_tokens.sql
/// The type of token that was created.
pub type AccountTokenType = AccountTokenOperation;

// IMPORTANT: keep this in sync with the ty enum in src/setup/mysql/credential_auth_tokens.sql
/// The type of token that was created.
pub type CredentialAuthTokenType = CredentialAuthTokenOperation;
