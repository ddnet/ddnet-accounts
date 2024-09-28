use ddnet_accounts_shared::client::{logout_all, machine_id::machine_uid};
use thiserror::Error;

use crate::{
    errors::{FsLikeError, HttpLikeError},
    interface::Io,
    safe_interface::{IoSafe, SafeIo},
};

/// The result of a [`logout_all`] request.
#[derive(Error, Debug)]
pub enum LogoutAllResult {
    /// A http like error occurred.
    #[error("{0}")]
    HttpLikeError(HttpLikeError),
    /// A fs like error occurred.
    #[error("{0}")]
    FsLikeError(FsLikeError),
    /// Errors that are not handled explicitly.
    #[error("Delete failed: {0}")]
    Other(anyhow::Error),
}

impl From<HttpLikeError> for LogoutAllResult {
    fn from(value: HttpLikeError) -> Self {
        Self::HttpLikeError(value)
    }
}

impl From<FsLikeError> for LogoutAllResult {
    fn from(value: FsLikeError) -> Self {
        Self::FsLikeError(value)
    }
}

/// Delete all sessions of an account on the account server, except
/// for the current one.
pub async fn logout_all(
    account_token_hex: String,
    io: &dyn Io,
) -> anyhow::Result<(), LogoutAllResult> {
    logout_all_impl(account_token_hex, io.into()).await
}

async fn logout_all_impl(
    account_token_hex: String,
    io: IoSafe<'_>,
) -> anyhow::Result<(), LogoutAllResult> {
    // read session's key-pair
    let key_pair = io.read_serialized_session_key_pair().await?;

    let hashed_hw_id = machine_uid().map_err(LogoutAllResult::Other)?;

    let delete_req = logout_all::logout_all(
        account_token_hex,
        hashed_hw_id,
        &key_pair.private_key,
        key_pair.public_key,
    )
    .map_err(LogoutAllResult::Other)?;

    io.request_logout_all(delete_req)
        .await?
        .map_err(|err| LogoutAllResult::Other(err.into()))?;

    Ok(())
}
