use thiserror::Error;

/// An error that is similar to
/// common http errrors.
/// Used for requests to the account
/// server.
#[derive(Error, Debug)]
pub enum HttpLikeError {
    /// The request failed.
    #[error("The request failed to be sent.")]
    Request,
    /// Http-like status codes.
    #[error("The server responsed with status code {0}")]
    Status(u16),
    /// Other errors
    #[error("{0}")]
    Other(anyhow::Error),
}

impl From<serde_json::Error> for HttpLikeError {
    fn from(value: serde_json::Error) -> Self {
        Self::Other(value.into())
    }
}

/// An error that is similar to
/// a file system error.
#[derive(Error, Debug)]
pub enum FsLikeError {
    /// The request failed.
    #[error("{0}")]
    Fs(std::io::Error),
    /// Other errors
    #[error("{0}")]
    Other(anyhow::Error),
}

impl From<std::io::Error> for FsLikeError {
    fn from(value: std::io::Error) -> Self {
        Self::Fs(value)
    }
}
