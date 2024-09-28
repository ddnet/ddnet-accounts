use ed25519_dalek::{SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};

use super::machine_id::{machine_uid, MachineUid};

/// This is the account data that should be sent to the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountDataForServer {
    /// The public key for this session.
    /// Used to verify the ownership of
    /// the key pair on the account server.
    pub public_key: VerifyingKey,
    /// A unique identifier that is used
    /// to verify the user's ownership
    /// for the key pair as an additional
    /// security enhancement.
    pub hw_id: MachineUid,
}

/// The key pair for the client
/// for the given account.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountDataForClient {
    /// A ed25519 private key that is used to to generate
    /// a signature to identify the user's session on the account server.  
    /// __!WARNING!__: Never share this key with anyone. It's only intended
    /// to be stored on __one__ of the client's computer. And not even shared between two
    /// computers of the same person.
    pub private_key: SigningKey,
    /// A ed25519 public key, which is sent to the account server and signed
    /// to auth the user's session.
    pub public_key: VerifyingKey,
}

/// The result type for [`generate_account_data`].
/// Contains everything that is required to register a new account
/// or to change a password on client & server.
#[derive(Debug)]
pub struct AccountData {
    /// Data that should be send to the server, see [`AccountDataForServer`]
    pub for_server: AccountDataForServer,
    /// Data that should be kept secret on the client, see [`AccountDataForClient`]
    pub for_client: AccountDataForClient,
}

/// Generates a new key pair based on ed25519 curve.
pub fn key_pair() -> (SigningKey, VerifyingKey) {
    // This key-pair is similar to a session token for an account
    // The client "registers" a pub-key on the account server,
    // which the account server uses to identify the client's
    // session private key.
    // Additionally the account server generates certificates for
    // this public key to proof they correlate to an existing
    // account.
    let mut rng = rand::rngs::OsRng;
    let private_key = SigningKey::generate(&mut rng);
    let public_key = private_key.verifying_key();
    (private_key, public_key)
}

/// This generates new account data from a key pair from an existing key-pair.
///
/// # Errors
/// Only returns errors if one of the crypto functions
/// failed to execute.
pub fn generate_account_data_from_key_pair(
    private_key: SigningKey,
    public_key: VerifyingKey,
) -> anyhow::Result<AccountData> {
    Ok(AccountData {
        for_server: AccountDataForServer {
            public_key,
            hw_id: machine_uid()?,
        },
        for_client: AccountDataForClient {
            private_key,
            public_key,
        },
    })
}

/// This generates new account data from a key pair.
///
/// # Errors
/// Only returns errors if one of the crypto functions
/// failed to execute.
pub fn generate_account_data() -> anyhow::Result<AccountData> {
    let (private_key, public_key) = key_pair();

    generate_account_data_from_key_pair(private_key, public_key)
}
