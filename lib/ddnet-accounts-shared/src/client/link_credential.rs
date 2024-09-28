use anyhow::anyhow;
use serde::{Deserialize, Serialize};

use super::{account_token::AccountToken, login::CredentialAuthToken};

/// Represents the data required for a delete attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkCredentialRequest {
    /// An account token that is used to verify that the delete
    /// request is valid.
    pub account_token: AccountToken,
    /// Data for the credential specific type,
    /// e.g. the email address or steamid.
    pub credential_auth_token: CredentialAuthToken,
}

/// Prepares a link credential request for the account server.
pub fn link_credential(
    account_token_hex: String,
    credential_auth_token_hex: String,
) -> anyhow::Result<LinkCredentialRequest> {
    let account_token = hex::decode(account_token_hex)?;
    let credential_auth_token = hex::decode(credential_auth_token_hex)?;

    Ok(LinkCredentialRequest {
        account_token: account_token
            .try_into()
            .map_err(|_| anyhow!("Invalid account token."))?,
        credential_auth_token: credential_auth_token
            .try_into()
            .map_err(|_| anyhow!("Invalid credential auth token."))?,
    })
}
