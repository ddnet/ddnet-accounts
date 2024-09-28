use ddnet_accounts_shared::{
    account_server::account_info::AccountInfoResponse,
    client::{account_info::prepare_account_info_request, machine_id::machine_uid},
};
use thiserror::Error;

use crate::{
    errors::{FsLikeError, HttpLikeError},
    interface::Io,
    safe_interface::{IoSafe, SafeIo},
};

/// The result of a [`account_info`] request.
#[derive(Error, Debug)]
pub enum AccountInfoResult {
    /// Session was invalid, must login again.
    #[error("The session was not valid anymore.")]
    SessionWasInvalid,
    /// A file system like error occurred.
    /// This usually means the user was not yet logged in.
    #[error("{0}")]
    FsLikeError(FsLikeError),
    /// A http like error occurred.
    #[error("{0}")]
    HttpLikeError(HttpLikeError),
    /// Errors that are not handled explicitly.
    #[error("Fetching account info failed: {0}")]
    Other(anyhow::Error),
}

impl From<HttpLikeError> for AccountInfoResult {
    fn from(value: HttpLikeError) -> Self {
        Self::HttpLikeError(value)
    }
}

impl From<FsLikeError> for AccountInfoResult {
    fn from(value: FsLikeError) -> Self {
        Self::FsLikeError(value)
    }
}

/// Fetches the account info of an account for an existing session on the account server.
///
/// # Errors
///
/// If an error occurs this usually means that the session is not valid anymore.
pub async fn account_info(io: &dyn Io) -> anyhow::Result<AccountInfoResponse, AccountInfoResult> {
    account_info_impl(io.into()).await
}

async fn account_info_impl(
    io: IoSafe<'_>,
) -> anyhow::Result<AccountInfoResponse, AccountInfoResult> {
    // read session's key-pair
    let key_pair = io.read_serialized_session_key_pair().await?;

    let hashed_hw_id = machine_uid().map_err(AccountInfoResult::Other)?;

    // do the account info request using the above private key
    let msg =
        prepare_account_info_request(hashed_hw_id, &key_pair.private_key, key_pair.public_key);
    io.request_account_info(msg)
        .await?
        .map_err(|err| AccountInfoResult::Other(err.into()))
}
