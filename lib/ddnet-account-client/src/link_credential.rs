use ddnet_accounts_shared::{
    account_server::errors::{AccountServerRequestError, Empty},
    client::link_credential::{self},
};

use thiserror::Error;

use crate::{
    errors::{FsLikeError, HttpLikeError},
    interface::Io,
    safe_interface::{IoSafe, SafeIo},
};

/// The result of a [`link_credential`] request.
#[derive(Error, Debug)]
pub enum LinkCredentialResult {
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
    #[error("Linking credential failed: {0}")]
    Other(anyhow::Error),
}

impl From<HttpLikeError> for LinkCredentialResult {
    fn from(value: HttpLikeError) -> Self {
        Self::HttpLikeError(value)
    }
}

impl From<FsLikeError> for LinkCredentialResult {
    fn from(value: FsLikeError) -> Self {
        Self::FsLikeError(value)
    }
}

/// Link another crendential to an account.
pub async fn link_credential(
    account_token_hex: String,
    credential_auth_token_hex: String,
    io: &dyn Io,
) -> anyhow::Result<(), LinkCredentialResult> {
    link_credential_impl(account_token_hex, credential_auth_token_hex, io.into()).await
}

async fn link_credential_impl(
    account_token_hex: String,
    credential_auth_token_hex: String,
    io: IoSafe<'_>,
) -> anyhow::Result<(), LinkCredentialResult> {
    io.request_link_credential(
        link_credential::link_credential(account_token_hex, credential_auth_token_hex)
            .map_err(LinkCredentialResult::Other)?,
    )
    .await?
    .map_err(LinkCredentialResult::AccountServerRequstError)?;

    Ok(())
}
