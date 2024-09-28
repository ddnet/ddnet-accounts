use serde::{Deserialize, Serialize};

/// The response of an sign request from the client.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignResponseSuccess {
    /// certificate, serialized in der format.
    pub cert_der: Vec<u8>,
}
