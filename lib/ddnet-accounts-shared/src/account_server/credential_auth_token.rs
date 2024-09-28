use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

/// The response of a credential auth token request by the client.
#[derive(Debug, Error, Clone, Serialize, Deserialize)]
pub enum CredentialAuthTokenError {
    /// Token invalid, probably timed out
    #[error("Because of spam you have to visit this web page to continue: {url}.")]
    WebValidationProcessNeeded {
        /// The url the client has to visit in order to continue
        url: Url,
    },
}
