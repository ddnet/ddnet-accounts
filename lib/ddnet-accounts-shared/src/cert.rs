use std::time::Duration;

use der::Decode;
use ed25519_dalek::{
    pkcs8::{spki::der::pem::LineEnding, EncodePrivateKey},
    SigningKey,
};
use rcgen::{CertificateParams, KeyPair, PKCS_ED25519};

pub use rcgen::CertifiedKey;

/// Generates a self signed certificate and key-pair as [`CertifiedKey`]
/// from a ed25519 private key.
pub fn generate_self_signed(private_key: &SigningKey) -> anyhow::Result<x509_cert::Certificate> {
    let key = private_key.to_pkcs8_pem(LineEnding::LF)?;
    let key_pair = KeyPair::from_pkcs8_pem_and_sign_algo(&key, &PKCS_ED25519)?;
    let mut cert_params = CertificateParams::new(vec!["localhost".into()])?;
    let now = std::time::SystemTime::now();
    cert_params.not_before = (now.checked_sub(Duration::from_secs(60 * 10)))
        .unwrap_or(now)
        .into();
    cert_params.not_after = (now + Duration::from_secs(60 * 60 * 4)).into();
    let cert = cert_params.self_signed(&key_pair)?;

    // yep, this is stupid, didn't get x509_cert to work with ed25519 keys
    Ok(x509_cert::Certificate::from_der(cert.der())?)
}
