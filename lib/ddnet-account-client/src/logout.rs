use ddnet_accounts_shared::client::{logout::prepare_logout_request, machine_id::machine_uid};
use thiserror::Error;

use crate::{
    errors::{FsLikeError, HttpLikeError},
    interface::Io,
    safe_interface::{IoSafe, SafeIo},
};

/// The result of a [`logout`] request.
#[derive(Error, Debug)]
pub enum LogoutResult {
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
    #[error("Logging out failed: {0}")]
    Other(anyhow::Error),
}

impl From<HttpLikeError> for LogoutResult {
    fn from(value: HttpLikeError) -> Self {
        Self::HttpLikeError(value)
    }
}

impl From<FsLikeError> for LogoutResult {
    fn from(value: FsLikeError) -> Self {
        Self::FsLikeError(value)
    }
}

/// Log out an existing session on the account server.
///
/// # Errors
///
/// If an error occurs this usually means that the session is not valid anymore.
pub async fn logout(io: &dyn Io) -> anyhow::Result<(), LogoutResult> {
    logout_impl(io.into()).await
}

async fn logout_impl(io: IoSafe<'_>) -> anyhow::Result<(), LogoutResult> {
    // read session's key-pair
    let key_pair = io.read_serialized_session_key_pair().await?;

    let hashed_hw_id = machine_uid().map_err(LogoutResult::Other)?;

    // do the logout request using the above private key
    let msg = prepare_logout_request(hashed_hw_id, &key_pair.private_key, key_pair.public_key);
    io.request_logout(msg)
        .await?
        .map_err(|err| LogoutResult::Other(err.into()))?;

    // remove the session's key pair
    io.remove_serialized_session_key_pair().await?;
    Ok(())
}
