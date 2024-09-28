use serde::{Deserialize, Serialize};
use thiserror::Error;

/// The response of a login request by the client.
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum LoginError {
    /// Token invalid, probably timed out
    #[error("The provided token is not valid anymore.")]
    TokenInvalid,
}
