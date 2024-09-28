use anyhow::anyhow;
use chrono::{DateTime, Utc};
use ecdsa::signature::Signer;
use ed25519_dalek::{Signature, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};

use super::{
    account_data::AccountDataForServer, account_token::AccountToken, machine_id::MachineUid,
};

/// Represents a session that is ignored
/// during a logout all attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IgnoreSession {
    /// The account data required to verify the session on the account server.
    pub account_data: AccountDataForServer,
    /// The timestamp when the logout request was triggered
    pub time_stamp: DateTime<Utc>,
    /// The signature for the above time stamp
    pub signature: Signature,
}

/// Represents the data required for a logout all attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoutAllRequest {
    /// An account token that is used to verify that the delete
    /// request is valid.
    pub account_token: AccountToken,

    /// Optionally a session can be ignored during logout.
    /// So this logout all is basically a logout all others.
    pub ignore_session: Option<IgnoreSession>,
}

/// Prepares a logout all request for the account server.
pub fn logout_all(
    account_token_hex: String,

    hw_id: MachineUid,
    key: &SigningKey,
    pub_key: VerifyingKey,
) -> anyhow::Result<LogoutAllRequest> {
    let account_token = hex::decode(account_token_hex)?;

    let time_stamp = chrono::Utc::now();
    let time_str = time_stamp.to_string();

    let signature = key.sign(time_str.as_bytes());

    Ok(LogoutAllRequest {
        account_token: account_token
            .try_into()
            .map_err(|_| anyhow!("Invalid account token."))?,

        ignore_session: Some(IgnoreSession {
            account_data: AccountDataForServer {
                public_key: pub_key,
                hw_id,
            },
            signature,
            time_stamp,
        }),
    })
}
