use ddnet_accounts_shared::{
    account_server::{account_token::AccountTokenError, errors::AccountServerRequestError},
    client::account_token::{
        AccountTokenEmailRequest, AccountTokenOperation, AccountTokenSteamRequest, SecretKey,
    },
};

use anyhow::anyhow;
use thiserror::Error;

use crate::{
    errors::{FsLikeError, HttpLikeError},
    interface::Io,
    safe_interface::{IoSafe, SafeIo},
};

/// The result of a [`account`] request.
#[derive(Error, Debug)]
pub enum AccountTokenResult {
    /// A http like error occurred.
    #[error("{0}")]
    HttpLikeError(HttpLikeError),
    /// A fs like error occurred.
    #[error("{0}")]
    FsLikeError(FsLikeError),
    #[error("{0:?}")]
    /// The account server responded with an error.
    AccountServerRequstError(AccountServerRequestError<AccountTokenError>),
    /// Errors that are not handled explicitly.
    #[error("Account failed: {0}")]
    Other(anyhow::Error),
}

impl From<HttpLikeError> for AccountTokenResult {
    fn from(value: HttpLikeError) -> Self {
        Self::HttpLikeError(value)
    }
}

impl From<FsLikeError> for AccountTokenResult {
    fn from(value: FsLikeError) -> Self {
        Self::FsLikeError(value)
    }
}

fn get_secret_key(
    secret_key_hex: Option<String>,
) -> anyhow::Result<Option<SecretKey>, AccountTokenResult> {
    secret_key_hex
        .map(hex::decode)
        .transpose()
        .map_err(|err| AccountTokenResult::Other(err.into()))?
        .map(|secret_key| secret_key.try_into())
        .transpose()
        .map_err(|_| {
            AccountTokenResult::Other(anyhow!(
                "secret key had an invalid length. make sure you copied it correctly."
            ))
        })
}

/// Generate a token sent by email.
pub async fn account_token_email(
    email: email_address::EmailAddress,
    op: AccountTokenOperation,
    secret_key_hex: Option<String>,
    io: &dyn Io,
) -> anyhow::Result<(), AccountTokenResult> {
    account_token_email_impl(email, op, secret_key_hex, io.into()).await
}

async fn account_token_email_impl(
    email: email_address::EmailAddress,
    op: AccountTokenOperation,
    secret_key_hex: Option<String>,
    io: IoSafe<'_>,
) -> anyhow::Result<(), AccountTokenResult> {
    if secret_key_hex.is_some() {
        io.request_account_token_email_secret(AccountTokenEmailRequest {
            email,
            secret_key: get_secret_key(secret_key_hex)?,
            op,
        })
        .await
    } else {
        io.request_account_token_email(AccountTokenEmailRequest {
            email,
            secret_key: get_secret_key(secret_key_hex)?,
            op,
        })
        .await
    }?
    .map_err(AccountTokenResult::AccountServerRequstError)?;

    Ok(())
}

/// Request an account token for the given steam credential.
/// The token is serialized in hex.
pub async fn account_token_steam(
    steam_ticket: Vec<u8>,
    op: AccountTokenOperation,
    secret_key_hex: Option<String>,
    io: &dyn Io,
) -> anyhow::Result<String, AccountTokenResult> {
    account_token_steam_impl(steam_ticket, op, secret_key_hex, io.into()).await
}

async fn account_token_steam_impl(
    steam_ticket: Vec<u8>,
    op: AccountTokenOperation,
    secret_key_hex: Option<String>,
    io: IoSafe<'_>,
) -> anyhow::Result<String, AccountTokenResult> {
    let account_token_hex = if secret_key_hex.is_some() {
        io.request_account_token_steam_secret(AccountTokenSteamRequest {
            steam_ticket,
            secret_key: get_secret_key(secret_key_hex)?,
            op,
        })
        .await
    } else {
        io.request_account_token_steam(AccountTokenSteamRequest {
            steam_ticket,
            secret_key: get_secret_key(secret_key_hex)?,
            op,
        })
        .await
    }?
    .map_err(AccountTokenResult::AccountServerRequstError)?;

    Ok(account_token_hex)
}
