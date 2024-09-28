use chrono::{DateTime, Utc};
use ed25519_dalek::{ed25519::signature::Signer, Signature, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};

use super::{account_data::AccountDataForServer, machine_id::MachineUid};

/// Represents an auth request the client
/// sends to the account server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignRequest {
    /// The account data required to verify the user on the account server.
    pub account_data: AccountDataForServer,
    /// The timestamp when the sign request was triggered
    pub time_stamp: DateTime<Utc>,
    /// The signature for the above time stamp
    pub signature: Signature,
}

/// Generate data for an sign request
pub fn prepare_sign_request(
    hw_id: MachineUid,
    key: &SigningKey,
    pub_key: VerifyingKey,
) -> SignRequest {
    let time_stamp = chrono::Utc::now();
    let time_str = time_stamp.to_string();

    let signature = key.sign(time_str.as_bytes());

    SignRequest {
        account_data: AccountDataForServer {
            public_key: pub_key,
            hw_id,
        },
        signature,
        time_stamp,
    }
}
