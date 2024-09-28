pub mod queries;

use std::sync::Arc;

use ddnet_account_sql::query::Query;
use ddnet_accounts_shared::{
    account_server::{
        errors::{AccountServerRequestError, Empty},
        result::AccountServerReqResult,
    },
    client::logout_all::{IgnoreSession, LogoutAllRequest},
};
use axum::Json;
use sqlx::{Acquire, AnyPool, Connection};

use crate::{
    account_token::queries::{AccountTokenQry, InvalidateAccountToken},
    shared::{Shared, CERT_MAX_AGE_DELTA, CERT_MIN_AGE_DELTA},
    types::AccountTokenType,
};

use self::queries::RemoveSessionsExcept;

pub async fn logout_all_request(
    shared: Arc<Shared>,
    pool: AnyPool,
    Json(data): Json<LogoutAllRequest>,
) -> Json<AccountServerReqResult<(), Empty>> {
    Json(logout_all(shared, pool, data).await.map_err(|err| {
        AccountServerRequestError::Unexpected {
            target: "logout_all".into(),
            err: err.to_string(),
            bt: err.backtrace().to_string(),
        }
    }))
}

pub async fn logout_all(
    shared: Arc<Shared>,
    pool: AnyPool,
    data: LogoutAllRequest,
) -> anyhow::Result<()> {
    let mut connection = pool.acquire().await?;
    let connection = connection.acquire().await?;

    connection
        .transaction(|connection| {
            Box::pin(async move {
                // token data
                let acc_token_qry = AccountTokenQry {
                    token: &data.account_token,
                };

                let row = acc_token_qry
                    .query(&shared.db.account_token_qry_statement)
                    .fetch_one(&mut **connection)
                    .await?;

                let token_data = AccountTokenQry::row_data(&row)?;

                // invalidate token
                let qry = InvalidateAccountToken {
                    token: &data.account_token,
                };
                qry.query(&shared.db.invalidate_account_token_statement)
                    .execute(&mut **connection)
                    .await?;

                anyhow::ensure!(
                    token_data.ty == AccountTokenType::LogoutAll,
                    "Account token was not for logout all operation."
                );
                let account_id = token_data.account_id;

                let validate_session = |ignore_session: IgnoreSession| {
                    ignore_session.account_data.public_key.verify_strict(
                        ignore_session.time_stamp.to_string().as_bytes(),
                        &ignore_session.signature,
                    )?;
                    let now = chrono::Utc::now();
                    let delta = now.signed_duration_since(ignore_session.time_stamp);
                    anyhow::ensure!(
                        delta < CERT_MAX_AGE_DELTA && delta > CERT_MIN_AGE_DELTA,
                        "time stamp was not in a valid time frame."
                    );
                    anyhow::Ok(ignore_session.account_data)
                };
                let session_data = data.ignore_session.and_then(|ignore_session| {
                    // if validating fails, still log out all sessions, since that is less important
                    validate_session(ignore_session).ok()
                });

                // remove all sessions
                let qry = RemoveSessionsExcept {
                    account_id: &account_id,
                    session_data: &session_data,
                };

                qry.query(&shared.db.remove_sessions_except_statement)
                    .execute(&mut **connection)
                    .await?;

                anyhow::Ok(())
            })
        })
        .await?;

    Ok(())
}
