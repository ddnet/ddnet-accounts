use rand::Rng;
use serde::{Deserialize, Serialize};

/// Represents an one time password, that the
/// account server uses to prevent replay attacks,
/// when clients are authing.
pub type Otp = [u8; 16];

/// Generates a new random one time password
pub fn generate_otp() -> Otp {
    rand::rngs::OsRng.gen::<Otp>()
}

/// The response to a client otp request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtpResponse {
    /// The one time passwords the client can use
    pub otps: Vec<Otp>,
}
