use ddnet_accounts_shared::{
    account_server::certs::AccountServerCertificates, game_server::user_id::VerifyingKey,
};
use anyhow::anyhow;
use x509_cert::{
    der::{Decode, Encode},
    spki::DecodePublicKey,
};

use crate::{
    errors::HttpLikeError,
    interface::Io,
    safe_interface::{IoSafe, SafeIo},
};

/// Downloads the latest legit account server certificates that are used to verify client
/// certificates signed by the account server
pub async fn download_certs(
    io: &dyn Io,
) -> anyhow::Result<AccountServerCertificates, HttpLikeError> {
    download_certs_impl(io.into()).await
}

async fn download_certs_impl(
    io: IoSafe<'_>,
) -> anyhow::Result<AccountServerCertificates, HttpLikeError> {
    let certs = io.download_account_server_certificates().await?;
    certs
        .map_err(|err| HttpLikeError::Other(err.into()))?
        .into_iter()
        .map(|cert| x509_cert::Certificate::from_der(&cert).map_err(|err| anyhow!(err)))
        .collect::<anyhow::Result<Vec<_>>>()
        .map_err(HttpLikeError::Other)
}

/// Extract the public key from certificates
pub fn certs_to_pub_keys(certs: &[x509_cert::Certificate]) -> Vec<VerifyingKey> {
    certs
        .iter()
        .flat_map(|cert| {
            cert.tbs_certificate
                .subject_public_key_info
                .to_der()
                .ok()
                .and_then(|v| VerifyingKey::from_public_key_der(&v).ok())
        })
        .collect::<Vec<_>>()
}
