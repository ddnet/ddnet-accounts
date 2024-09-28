use anyhow::anyhow;
use serde::{Deserialize, Serialize};

use super::account_token::AccountToken;

/// Represents the data required for a delete attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteRequest {
    /// An account token that is used to verify that the delete
    /// request is valid.
    pub account_token: AccountToken,
}

/// Prepares a delete request for the account server.
pub fn delete(account_token_hex: String) -> anyhow::Result<DeleteRequest> {
    let account_token = hex::decode(account_token_hex)?;

    Ok(DeleteRequest {
        account_token: account_token
            .try_into()
            .map_err(|_| anyhow!("Invalid account token."))?,
    })
}
