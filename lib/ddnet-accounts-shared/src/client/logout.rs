use chrono::{DateTime, Utc};
use ed25519_dalek::{ed25519::signature::Signer, Signature, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};

use super::{account_data::AccountDataForServer, machine_id::MachineUid};

/// Represents the data required for a logout attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoutRequest {
    /// The account data related to the logout request.
    pub account_data: AccountDataForServer,
    /// The timestamp when the sign request was triggered
    pub time_stamp: DateTime<Utc>,
    /// The signature for the above time stamp
    pub signature: Signature,
}

/// Generate data for an logout request
pub fn prepare_logout_request(
    hw_id: MachineUid,
    key: &SigningKey,
    pub_key: VerifyingKey,
) -> LogoutRequest {
    let time_stamp = chrono::Utc::now();
    let time_str = time_stamp.to_string();

    let signature = key.sign(time_str.as_bytes());

    LogoutRequest {
        account_data: AccountDataForServer {
            public_key: pub_key,
            hw_id,
        },
        signature,
        time_stamp,
    }
}
