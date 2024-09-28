pub mod queries;

use std::sync::Arc;

use ddnet_account_sql::query::Query;
use ddnet_accounts_shared::{
    account_server::{
        errors::{AccountServerRequestError, Empty},
        result::AccountServerReqResult,
    },
    client::logout::LogoutRequest,
};
use axum::Json;
use sqlx::{Acquire, AnyPool};

use crate::shared::{Shared, CERT_MAX_AGE_DELTA, CERT_MIN_AGE_DELTA};

use self::queries::RemoveSession;

pub async fn logout_request(
    shared: Arc<Shared>,
    pool: AnyPool,
    Json(data): Json<LogoutRequest>,
) -> Json<AccountServerReqResult<(), Empty>> {
    Json(
        logout(shared, pool, data)
            .await
            .map_err(|err| AccountServerRequestError::Unexpected {
                target: "logout".into(),
                err: err.to_string(),
                bt: err.backtrace().to_string(),
            }),
    )
}

pub async fn logout(shared: Arc<Shared>, pool: AnyPool, data: LogoutRequest) -> anyhow::Result<()> {
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
    let connection = connection.acquire().await?;

    // remove this session
    let qry = RemoveSession {
        pub_key: data.account_data.public_key.as_bytes(),
        hw_id: &data.account_data.hw_id,
    };

    qry.query(&shared.db.logout_statement)
        .execute(connection)
        .await?;

    Ok(())
}
