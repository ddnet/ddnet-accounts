use ddnet_accounts_shared::{
    account_server::{errors::AccountServerRequestError, login::LoginError},
    client::{
        account_data::AccountDataForClient,
        login::{self, LoginRequest},
    },
};
use ddnet_accounts_types::account_id::AccountId;
use thiserror::Error;

use crate::{
    errors::{FsLikeError, HttpLikeError},
    interface::Io,
    safe_interface::{IoSafe, SafeIo},
};

/// The result of a [`login`] request.
#[derive(Error, Debug)]
pub enum LoginResult {
    /// A http like error occurred.
    #[error("{0}")]
    HttpLikeError(HttpLikeError),
    /// A fs like error occurred.
    #[error("{0}")]
    FsLikeError(FsLikeError),
    /// The account server responded with an error.
    #[error("{0}")]
    AccountServerRequstError(AccountServerRequestError<LoginError>),
    /// Errors that are not handled explicitly.
    #[error("Login failed: {0}")]
    Other(anyhow::Error),
}

impl From<HttpLikeError> for LoginResult {
    fn from(value: HttpLikeError) -> Self {
        Self::HttpLikeError(value)
    }
}

impl From<FsLikeError> for LoginResult {
    fn from(value: FsLikeError) -> Self {
        Self::FsLikeError(value)
    }
}

/// Writes the session data to disk
#[must_use = "This writes the login data and must be used by calling \"write\" of this object"]
#[derive(Debug)]
pub struct LoginWriter {
    account_data: AccountDataForClient,
}

impl LoginWriter {
    /// Writes the session data to disk
    async fn write_impl(self, io: IoSafe<'_>) -> anyhow::Result<(), FsLikeError> {
        io.write_serialized_session_key_pair(&self.account_data)
            .await
    }

    /// Writes the session data to disk
    pub async fn write(self, io: &dyn Io) -> anyhow::Result<(), FsLikeError> {
        self.write_impl(io.into()).await
    }
}

async fn login_inner_impl(
    login_req: LoginRequest,
    login_data: AccountDataForClient,
    io: IoSafe<'_>,
) -> anyhow::Result<(AccountId, LoginWriter), LoginResult> {
    let account_id = io
        .request_login(login_req)
        .await?
        .map_err(LoginResult::AccountServerRequstError)?;

    Ok((
        account_id,
        LoginWriter {
            account_data: login_data,
        },
    ))
}

/// Create a new session (or account if not existed) on the account server.
pub async fn login_with_account_data(
    credential_auth_token_hex: String,
    account_data: &AccountDataForClient,
    io: &dyn Io,
) -> anyhow::Result<(AccountId, LoginWriter), LoginResult> {
    login_with_account_data_impl(credential_auth_token_hex, account_data, io.into()).await
}

async fn login_with_account_data_impl(
    credential_auth_token_hex: String,
    account_data: &AccountDataForClient,
    io: IoSafe<'_>,
) -> anyhow::Result<(AccountId, LoginWriter), LoginResult> {
    let (login_req, login_data) =
        login::login_from_client_account_data(account_data, credential_auth_token_hex)
            .map_err(LoginResult::Other)?;

    login_inner_impl(login_req, login_data, io).await
}

/// Create a new session (or account if not existed) on the account server.
pub async fn login(
    credential_auth_token_hex: String,
    io: &dyn Io,
) -> anyhow::Result<(AccountId, LoginWriter), LoginResult> {
    login_impl(credential_auth_token_hex, io.into()).await
}

async fn login_impl(
    credential_auth_token_hex: String,
    io: IoSafe<'_>,
) -> anyhow::Result<(AccountId, LoginWriter), LoginResult> {
    let (login_req, login_data) =
        login::login(credential_auth_token_hex).map_err(LoginResult::Other)?;

    login_inner_impl(login_req, login_data, io).await
}
