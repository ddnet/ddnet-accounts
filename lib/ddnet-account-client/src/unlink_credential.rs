use ddnet_accounts_shared::{
    account_server::errors::{AccountServerRequestError, Empty},
    client::unlink_credential,
};

use thiserror::Error;

use crate::{
    errors::{FsLikeError, HttpLikeError},
    interface::Io,
    safe_interface::{IoSafe, SafeIo},
};

/// The result of a [`unlink_credential`] request.
#[derive(Error, Debug)]
pub enum UnlinkCredentialResult {
    /// A http like error occurred.
    #[error("{0}")]
    HttpLikeError(HttpLikeError),
    /// A fs like error occurred.
    #[error("{0}")]
    FsLikeError(FsLikeError),
    /// The account server responded with an error.
    #[error("{0}")]
    AccountServerRequstError(AccountServerRequestError<Empty>),
    /// Errors that are not handled explicitly.
    #[error("Unlinking credential failed: {0}")]
    Other(anyhow::Error),
}

impl From<HttpLikeError> for UnlinkCredentialResult {
    fn from(value: HttpLikeError) -> Self {
        Self::HttpLikeError(value)
    }
}

impl From<FsLikeError> for UnlinkCredentialResult {
    fn from(value: FsLikeError) -> Self {
        Self::FsLikeError(value)
    }
}

/// Unlink a credential from an account.
/// If the credential is the last one linked, this function fails.
pub async fn unlink_credential(
    credential_auth_token_hex: String,
    io: &dyn Io,
) -> anyhow::Result<(), UnlinkCredentialResult> {
    unlink_credential_impl(credential_auth_token_hex, io.into()).await
}

async fn unlink_credential_impl(
    credential_auth_token_hex: String,
    io: IoSafe<'_>,
) -> anyhow::Result<(), UnlinkCredentialResult> {
    io.request_unlink_credential(
        unlink_credential::unlink_credential(credential_auth_token_hex)
            .map_err(UnlinkCredentialResult::Other)?,
    )
    .await?
    .map_err(UnlinkCredentialResult::AccountServerRequstError)?;

    Ok(())
}
