use anyhow::anyhow;
use ed25519_dalek::{Signature, Signer};
use serde::{Deserialize, Serialize};

use crate::account_server::otp::Otp;

use super::account_data::{
    generate_account_data, generate_account_data_from_key_pair, AccountData, AccountDataForClient,
    AccountDataForServer,
};

/// A credential auth token previously sent to email or generated
/// for a steam login attempt.
pub type CredentialAuthToken = Otp;

/// Represents the data required for a login attempt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest {
    /// The account data related to the login request.
    pub account_data: AccountDataForServer,
    /// A credential auth token that was sent by
    /// email or generated for a steam based login etc.
    pub credential_auth_token: CredentialAuthToken,
    /// The signature for the credential auth token,
    /// used to make sure the public key corresponds
    /// to a valid private key.
    pub credential_auth_token_signature: Signature,
}

fn login_from_account_data(
    account_data: AccountData,
    credential_auth_token_hex: String,
) -> anyhow::Result<(LoginRequest, AccountDataForClient)> {
    let credential_auth_token = hex::decode(credential_auth_token_hex)?;
    let signature = account_data
        .for_client
        .private_key
        .sign(&credential_auth_token);

    Ok((
        LoginRequest {
            credential_auth_token_signature: signature,
            account_data: account_data.for_server,
            credential_auth_token: credential_auth_token
                .try_into()
                .map_err(|_| anyhow!("Invalid credential auth token."))?,
        },
        account_data.for_client,
    ))
}

/// Prepares a login request for the account server.
pub fn login_from_client_account_data(
    account_data: &AccountDataForClient,
    credential_auth_token_hex: String,
) -> anyhow::Result<(LoginRequest, AccountDataForClient)> {
    let account_data = generate_account_data_from_key_pair(
        account_data.private_key.clone(),
        account_data.public_key,
    )?;

    login_from_account_data(account_data, credential_auth_token_hex)
}

/// Prepares a login request for the account server.
pub fn login(
    credential_auth_token_hex: String,
) -> anyhow::Result<(LoginRequest, AccountDataForClient)> {
    let account_data = generate_account_data()?;

    login_from_account_data(account_data, credential_auth_token_hex)
}
