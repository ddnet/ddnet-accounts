use anyhow::anyhow;
use serde::{Deserialize, Serialize};

use super::login::CredentialAuthToken;

/// Represents the data required for a delete attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlinkCredentialRequest {
    /// Data for the credential specific type,
    /// e.g. the email address or steamid.
    pub credential_auth_token: CredentialAuthToken,
}

/// Prepares an unlink credential request for the account server.
pub fn unlink_credential(
    credential_auth_token_hex: String,
) -> anyhow::Result<UnlinkCredentialRequest> {
    let credential_auth_token = hex::decode(credential_auth_token_hex)?;

    Ok(UnlinkCredentialRequest {
        credential_auth_token: credential_auth_token
            .try_into()
            .map_err(|_| anyhow!("Invalid credential auth token."))?,
    })
}
