pub mod queries;

use std::{str::FromStr, sync::Arc, time::Duration};

use axum::Json;
use ddnet_account_sql::{any::AnyPool, query::Query};
use ddnet_accounts_shared::{
    account_server::{
        cert_account_ext::{AccountCertData, AccountCertExt},
        errors::{AccountServerRequestError, Empty},
        result::AccountServerReqResult,
        sign::SignResponseSuccess,
    },
    client::sign::SignRequest,
};
use p256::ecdsa::DerSignature;
use x509_cert::builder::Builder;
use x509_cert::der::Encode;
use x509_cert::{
    builder::Profile, name::Name, serial_number::SerialNumber, spki::SubjectPublicKeyInfoOwned,
    time::Validity,
};

use crate::shared::{Shared, CERT_MAX_AGE_DELTA, CERT_MIN_AGE_DELTA};

use self::queries::AuthAttempt;

pub async fn sign_request(
    shared: Arc<Shared>,
    pool: AnyPool,
    Json(data): Json<SignRequest>,
) -> Json<AccountServerReqResult<SignResponseSuccess, Empty>> {
    Json(
        sign(shared, pool, data)
            .await
            .map_err(|err| AccountServerRequestError::Unexpected {
                target: "sign".into(),
                err: err.to_string(),
                bt: err.backtrace().to_string(),
            }),
    )
}

pub async fn sign(
    shared: Arc<Shared>,
    pool: AnyPool,
    data: SignRequest,
) -> anyhow::Result<SignResponseSuccess> {
    data.account_data
        .public_key
        .verify_strict(data.time_stamp.to_string().as_bytes(), &data.signature)?;
    let now = chrono::Utc::now();
    let delta = now.signed_duration_since(data.time_stamp);
    anyhow::ensure!(
        delta < CERT_MAX_AGE_DELTA && delta > CERT_MIN_AGE_DELTA,
        "time stamp was not in a valid time frame."
    );

    let mut connection = pool.acquire().await?;
    let mut connection = connection.acquire().await?;

    let qry = AuthAttempt { data: &data };
    let row = qry
        .query(&shared.db.auth_attempt_statement)
        .fetch_one(&mut connection)
        .await?;
    let auth_data = AuthAttempt::row_data(&row)?;

    let serial_number = SerialNumber::from(42u32);
    let validity = Validity::from_now(Duration::new(60 * 60, 0))?;
    let profile = Profile::Root;
    let subject = Name::from_str("O=DDNet")?;

    let pub_key = SubjectPublicKeyInfoOwned::from_key(data.account_data.public_key)?;

    let signing_key = shared.signing_keys.read().clone();

    let mut builder = x509_cert::builder::CertificateBuilder::new(
        profile,
        serial_number,
        validity,
        subject,
        pub_key,
        &signing_key.current_key,
    )?;
    let unix_utc = auth_data
        .creation_date
        .signed_duration_since(sqlx::types::chrono::DateTime::UNIX_EPOCH);

    builder.add_extension(&AccountCertExt {
        data: AccountCertData {
            account_id: auth_data.account_id,
            utc_time_since_unix_epoch_millis: unix_utc.num_milliseconds(),
        },
    })?;
    let cert = builder.build::<DerSignature>()?.to_der()?;

    Ok(SignResponseSuccess { cert_der: cert })
}
