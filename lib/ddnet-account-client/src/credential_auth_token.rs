use ddnet_accounts_shared::{
    account_server::{
        credential_auth_token::CredentialAuthTokenError, errors::AccountServerRequestError,
    },
    client::credential_auth_token::{
        CredentialAuthTokenEmailRequest, CredentialAuthTokenOperation,
        CredentialAuthTokenSteamRequest, SecretKey,
    },
};

use anyhow::anyhow;
use thiserror::Error;

use crate::{
    errors::{FsLikeError, HttpLikeError},
    interface::Io,
    safe_interface::{IoSafe, SafeIo},
};

/// The result of a [`credential_auth_token_email`] request.
#[derive(Error, Debug)]
pub enum CredentialAuthTokenResult {
    /// A http like error occurred.
    #[error("{0}")]
    HttpLikeError(HttpLikeError),
    /// A fs like error occurred.
    #[error("{0}")]
    FsLikeError(FsLikeError),
    /// The account server responded with an error.
    #[error("{0}")]
    AccountServerRequstError(AccountServerRequestError<CredentialAuthTokenError>),
    /// Errors that are not handled explicitly.
    #[error("Credential authorization failed: {0}")]
    Other(anyhow::Error),
}

impl From<HttpLikeError> for CredentialAuthTokenResult {
    fn from(value: HttpLikeError) -> Self {
        Self::HttpLikeError(value)
    }
}

impl From<FsLikeError> for CredentialAuthTokenResult {
    fn from(value: FsLikeError) -> Self {
        Self::FsLikeError(value)
    }
}

fn get_secret_key(
    secret_key_hex: Option<String>,
) -> anyhow::Result<Option<SecretKey>, CredentialAuthTokenResult> {
    secret_key_hex
        .map(hex::decode)
        .transpose()
        .map_err(|err| CredentialAuthTokenResult::Other(err.into()))?
        .map(|secret_key| secret_key.try_into())
        .transpose()
        .map_err(|_| {
            CredentialAuthTokenResult::Other(anyhow!(
                "secret key had an invalid length. make sure you copied it correctly."
            ))
        })
}

/// Generate a token sent by email for a new session/account.
pub async fn credential_auth_token_email(
    email: email_address::EmailAddress,
    op: CredentialAuthTokenOperation,
    secret_key_hex: Option<String>,
    io: &dyn Io,
) -> anyhow::Result<(), CredentialAuthTokenResult> {
    credential_auth_token_email_impl(email, op, secret_key_hex, io.into()).await
}

async fn credential_auth_token_email_impl(
    email: email_address::EmailAddress,
    op: CredentialAuthTokenOperation,
    secret_key_hex: Option<String>,
    io: IoSafe<'_>,
) -> anyhow::Result<(), CredentialAuthTokenResult> {
    let secret_key = get_secret_key(secret_key_hex)?;
    if secret_key.is_some() {
        io.request_credential_auth_email_token_with_secret_key(CredentialAuthTokenEmailRequest {
            email,
            secret_key,
            op,
        })
        .await?
        .map_err(CredentialAuthTokenResult::AccountServerRequstError)?;
    } else {
        io.request_credential_auth_email_token(CredentialAuthTokenEmailRequest {
            email,
            secret_key,
            op,
        })
        .await?
        .map_err(CredentialAuthTokenResult::AccountServerRequstError)?;
    }

    Ok(())
}

/// Generate a token sent for a steam auth for a new session/account.
/// On success the credential auth token is returned in hex format.
pub async fn credential_auth_token_steam(
    steam_ticket: Vec<u8>,
    op: CredentialAuthTokenOperation,
    secret_key_hex: Option<String>,
    io: &dyn Io,
) -> anyhow::Result<String, CredentialAuthTokenResult> {
    credential_auth_token_steam_impl(steam_ticket, op, secret_key_hex, io.into()).await
}

async fn credential_auth_token_steam_impl(
    steam_ticket: Vec<u8>,
    op: CredentialAuthTokenOperation,
    secret_key_hex: Option<String>,
    io: IoSafe<'_>,
) -> anyhow::Result<String, CredentialAuthTokenResult> {
    let secret_key = get_secret_key(secret_key_hex)?;
    let credential_auth_token_hex = if secret_key.is_some() {
        io.request_credential_auth_steam_token_with_secret_key(CredentialAuthTokenSteamRequest {
            steam_ticket,
            secret_key,
            op,
        })
        .await?
        .map_err(CredentialAuthTokenResult::AccountServerRequstError)?
    } else {
        io.request_credential_auth_steam_token(CredentialAuthTokenSteamRequest {
            steam_ticket,
            secret_key,
            op,
        })
        .await?
        .map_err(CredentialAuthTokenResult::AccountServerRequstError)?
    };

    Ok(credential_auth_token_hex)
}
