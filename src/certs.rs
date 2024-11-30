pub mod queries;

use std::{str::FromStr, sync::Arc, time::Duration};

use anyhow::anyhow;
use axum::Json;
use ddnet_account_sql::{any::AnyPool, query::Query};
use ddnet_accounts_shared::account_server::{
    errors::{AccountServerRequestError, Empty},
    result::AccountServerReqResult,
};
use der::{Decode, Encode};
use p256::ecdsa::{DerSignature, SigningKey};
use queries::{AddCert, GetCerts};
use serde::{Deserialize, Serialize};
use x509_cert::{
    builder::{Builder, Profile},
    name::Name,
    serial_number::SerialNumber,
    spki::SubjectPublicKeyInfoOwned,
    time::Validity,
};

use crate::{db::DbConnectionShared, shared::Shared};

#[derive(Debug, Clone)]
pub struct PrivateKeys {
    pub current_key: SigningKey,
    pub current_cert: x509_cert::Certificate,
    pub next_key: SigningKey,
    pub next_cert: x509_cert::Certificate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PrivateKeysSer {
    pub current_key: Vec<u8>,
    pub current_cert: Vec<u8>,
    pub next_key: Vec<u8>,
    pub next_cert: Vec<u8>,
}

impl Serialize for PrivateKeys {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let current_key = self.current_key.to_bytes().to_vec();
        let current_cert = self
            .current_cert
            .to_der()
            .map_err(|_| serde::ser::Error::custom("cert to der failed"))?;
        let next_key = self.next_key.to_bytes().to_vec();
        let next_cert = self
            .next_cert
            .to_der()
            .map_err(|_| serde::ser::Error::custom("cert to der failed"))?;

        let keys = PrivateKeysSer {
            current_key,
            current_cert,
            next_key,
            next_cert,
        };

        keys.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for PrivateKeys {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let keys = <PrivateKeysSer>::deserialize(deserializer)?;

        Ok(Self {
            current_key: SigningKey::from_slice(&keys.current_key)
                .map_err(|_| serde::de::Error::custom("reading signing key from slice failed."))?,
            current_cert: x509_cert::Certificate::from_der(&keys.current_cert)
                .map_err(|_| serde::de::Error::custom("reading cert from slice failed."))?,
            next_key: SigningKey::from_slice(&keys.next_key)
                .map_err(|_| serde::de::Error::custom("reading signing key from slice failed."))?,
            next_cert: x509_cert::Certificate::from_der(&keys.next_cert)
                .map_err(|_| serde::de::Error::custom("reading cert from slice failed."))?,
        })
    }
}

pub fn generate_key_and_cert_impl(
    valid_for: Duration,
) -> anyhow::Result<(SigningKey, x509_cert::Certificate)> {
    let signing_key = SigningKey::random(&mut rand::rngs::OsRng);
    let verifying_key = signing_key.verifying_key();

    let serial_number = SerialNumber::from(42u32);
    let validity = Validity::from_now(valid_for)?;
    let profile = Profile::Root;
    let subject = Name::from_str("CN=DDNet,O=DDNet.org,C=EU")?;

    let pub_key = SubjectPublicKeyInfoOwned::from_key(*verifying_key)?;

    let cert = x509_cert::builder::CertificateBuilder::new(
        profile,
        serial_number,
        validity,
        subject,
        pub_key,
        &signing_key,
    )?
    .build::<DerSignature>()?;

    Ok((signing_key, cert))
}

pub fn generate_key_and_cert(
    first_key: bool,
) -> anyhow::Result<(SigningKey, x509_cert::Certificate)> {
    generate_key_and_cert_impl(Duration::new(
        if first_key { 1 } else { 2 } * 30 * 24 * 60 * 60,
        0,
    ))
}

pub async fn store_cert(
    db: &DbConnectionShared,
    pool: &AnyPool,
    cert: &x509_cert::Certificate,
) -> anyhow::Result<()> {
    let cert_der = cert.to_der()?;
    let time_stamp = cert
        .tbs_certificate
        .validity
        .not_after
        .to_date_time()
        .unix_duration();
    let valid_until = <sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>>::from_timestamp(
        time_stamp.as_secs() as i64,
        time_stamp.subsec_nanos(),
    )
    .ok_or_else(|| anyhow!("not a valid utc timestamp"))?;
    let qry = AddCert {
        cert_der: &cert_der,
        valid_until: &valid_until,
    };

    let mut connection = pool.acquire().await?;
    let mut connection = connection.acquire().await?;

    let res = qry
        .query(&db.add_cert_statement)
        .execute(&mut connection)
        .await?;
    anyhow::ensure!(res.rows_affected() >= 1);

    Ok(())
}

pub async fn get_certs(
    db: &DbConnectionShared,
    pool: &AnyPool,
) -> anyhow::Result<Vec<x509_cert::Certificate>> {
    let qry = GetCerts {};

    let mut connection = pool.acquire().await?;
    let mut connection = connection.acquire().await?;

    let cert_rows = qry
        .query(&db.get_certs_statement)
        .fetch_all(&mut connection)
        .await?;

    cert_rows
        .into_iter()
        .map(|row| GetCerts::row_data(&row))
        .collect::<anyhow::Result<Vec<_>>>()
        .and_then(|certs| {
            certs
                .into_iter()
                .map(|cert| {
                    x509_cert::Certificate::from_der(&cert.cert_der).map_err(|err| anyhow!(err))
                })
                .collect::<anyhow::Result<Vec<_>>>()
        })
}

pub async fn certs_request(
    shared: Arc<Shared>,
) -> Json<AccountServerReqResult<Vec<Vec<u8>>, Empty>> {
    let certs = shared.cert_chain.read().clone();
    Json(
        certs
            .iter()
            .map(|cert| cert.to_der().map_err(|err| anyhow!(err)))
            .collect::<anyhow::Result<Vec<_>>>()
            .map_err(|err| AccountServerRequestError::Unexpected {
                target: "certs_request".into(),
                err: err.to_string(),
                bt: err.backtrace().to_string(),
            }),
    )
}
