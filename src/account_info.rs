pub mod queries;

use std::{str::FromStr, sync::Arc};

use axum::Json;
use ddnet_account_sql::query::Query;
use ddnet_accounts_shared::{
    account_server::{
        account_info::{AccountInfoResponse, CredentialType},
        errors::{AccountServerRequestError, Empty},
        result::AccountServerReqResult,
    },
    client::account_info::AccountInfoRequest,
};
use queries::AccountInfo;
use sqlx::{Acquire, AnyPool};

use crate::shared::{Shared, CERT_MAX_AGE_DELTA, CERT_MIN_AGE_DELTA};

pub async fn account_info_request(
    shared: Arc<Shared>,
    pool: AnyPool,
    Json(data): Json<AccountInfoRequest>,
) -> Json<AccountServerReqResult<AccountInfoResponse, Empty>> {
    Json(account_info(shared, pool, data).await.map_err(|err| {
        AccountServerRequestError::Unexpected {
            target: "account_info".into(),
            err: err.to_string(),
            bt: err.backtrace().to_string(),
        }
    }))
}

pub async fn account_info(
    shared: Arc<Shared>,
    pool: AnyPool,
    data: AccountInfoRequest,
) -> anyhow::Result<AccountInfoResponse> {
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

    // fetch account info
    let qry = AccountInfo {
        session_pub_key: data.account_data.public_key.as_bytes(),
        session_hw_id: &data.account_data.hw_id,
    };

    let row = qry
        .query(connection, &shared.db.account_info)
        .fetch_one(connection)
        .await?;

    let account_info = AccountInfo::row_data(&row)?;
    Ok(AccountInfoResponse {
        account_id: account_info.account_id,
        creation_date: account_info.creation_date,
        credentials: account_info
            .linked_email
            .into_iter()
            .flat_map(|mail| {
                email_address::EmailAddress::from_str(&mail)
                    .ok()
                    .map(|mail| {
                        let repl_str = |str: &str| {
                            let str_count = str.chars().count();
                            let mut str_new = str
                                .chars()
                                .next()
                                .map(|c| c.to_string())
                                .unwrap_or_default();
                            str_new.extend((0..str_count.saturating_sub(2)).map(|_| '*'));
                            if str_count >= 2 {
                                if let Some(last) = str.chars().last() {
                                    str_new.push(last);
                                }
                            }
                            str_new
                        };
                        let local = repl_str(mail.local_part());
                        let (domain_name, domain_tld) = if let Some((domain_name, domain_tld)) =
                            mail.domain().split_once(".")
                        {
                            (repl_str(domain_name), format!(".{}", domain_tld))
                        } else {
                            (repl_str(mail.domain()), "".to_string())
                        };
                        CredentialType::Email(format!("{}@{}{}", local, domain_name, domain_tld))
                    })
            })
            .chain(
                account_info
                    .linked_steam
                    .into_iter()
                    .map(CredentialType::Steam),
            )
            .collect(),
    })
}
