use ddnet_accounts_shared::client::delete;
use thiserror::Error;

use crate::{
    errors::{FsLikeError, HttpLikeError},
    interface::Io,
    safe_interface::{IoSafe, SafeIo},
};

/// The result of a [`delete`] request.
#[derive(Error, Debug)]
pub enum DeleteResult {
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

impl From<HttpLikeError> for DeleteResult {
    fn from(value: HttpLikeError) -> Self {
        Self::HttpLikeError(value)
    }
}

impl From<FsLikeError> for DeleteResult {
    fn from(value: FsLikeError) -> Self {
        Self::FsLikeError(value)
    }
}

/// Delete an account on the account server.
pub async fn delete(account_token_hex: String, io: &dyn Io) -> anyhow::Result<(), DeleteResult> {
    delete_impl(account_token_hex, io.into()).await
}

async fn delete_impl(
    account_token_hex: String,
    io: IoSafe<'_>,
) -> anyhow::Result<(), DeleteResult> {
    let delete_req = delete::delete(account_token_hex).map_err(DeleteResult::Other)?;

    io.request_delete_account(delete_req)
        .await?
        .map_err(|err| DeleteResult::Other(err.into()))?;
    // this is generally allowed to fail
    let _ = io.remove_serialized_session_key_pair().await;

    Ok(())
}
