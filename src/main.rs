//! This is the http + db implementation for the account server.

#![deny(missing_docs)]
#![deny(warnings)]
#![deny(clippy::nursery)]
#![deny(clippy::all)]

pub(crate) mod account_token;
mod certs;
pub(crate) mod credential_auth_token;
pub(crate) mod db;
pub(crate) mod delete;
pub(crate) mod email;
pub(crate) mod login;
mod logout;
pub(crate) mod setup;
pub(crate) mod shared;
pub(crate) mod sign;
pub(crate) mod steam;
pub(crate) mod update;

pub(crate) mod email_limit;
pub(crate) mod ip_limit;

pub(crate) mod logout_all;

mod account_info;
mod file_watcher;
mod link_credential;
#[cfg(test)]
mod tests;
mod types;
mod unlink_credential;

use account_info::{account_info_request, queries::AccountInfo};
use account_token::{
    account_token_email, account_token_steam,
    queries::{
        AccountTokenQry, AddAccountTokenEmail, AddAccountTokenSteam, InvalidateAccountToken,
    },
};
use anyhow::anyhow;
use axum::{extract::DefaultBodyLimit, response::IntoResponse, Json, Router};
use certs::{
    certs_request, generate_key_and_cert, get_certs,
    queries::{AddCert, GetCerts},
    store_cert, PrivateKeys,
};
use clap::{command, parser::ValueSource, Arg, ArgAction};
use credential_auth_token::{
    credential_auth_token_email, credential_auth_token_steam, queries::AddCredentialAuthToken,
};
use db::DbConnectionShared;
use ddnet_account_sql::{any::AnyPool, query::Query};
use ddnet_accounts_shared::account_server::{
    errors::AccountServerRequestError, result::AccountServerReqResult,
};
use delete::{delete_request, queries::RemoveAccount};
use either::Either;
use email::EmailShared;
use ip_limit::{ip_deny_layer, IpDenyList};
use link_credential::{
    link_credential_request,
    queries::{UnlinkCredentialEmail, UnlinkCredentialSteam},
};
use login::{
    login_request,
    queries::{
        AccountIdFromEmail, AccountIdFromLastInsert, AccountIdFromSteam, CreateSession,
        CredentialAuthTokenQry, InvalidateCredentialAuthToken, LinkAccountCredentialEmail,
        LinkAccountCredentialSteam, TryCreateAccount,
    },
};
use logout::{logout_request, queries::RemoveSession};
use logout_all::{logout_all_request, queries::RemoveSessionsExcept};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use shared::Shared;
use sign::{queries::AuthAttempt, sign_request};
use sqlx::mysql::MySqlConnectOptions;
use sqlx::mysql::MySqlPoolOptions;
use std::{
    net::SocketAddr,
    num::NonZeroU32,
    path::PathBuf,
    sync::Arc,
    time::{Duration, SystemTime},
};
use steam::{SteamShared, OFFICIAL_STEAM_AUTH_URL};
use tokio::net::{TcpListener, TcpSocket};
use tower::ServiceBuilder;
use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor, GovernorLayer,
};
use unlink_credential::{
    queries::{UnlinkCredentialByEmail, UnlinkCredentialBySteam},
    unlink_credential_request,
};
use update::{
    handle_watchers,
    queries::{CleanupAccountTokens, CleanupCerts, CleanupCredentialAuthTokens},
    update,
};
use url::Url;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DbDetails {
    host: String,
    port: u16,
    database: String,
    username: String,
    password: String,
    ca_cert_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HttpServerDetails {
    port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EmailDetails {
    relay: String,
    relay_port: u16,
    username: String,
    password: String,
    /// The name of the sender of all emails
    /// e.g. `accounts@mydomain.org`
    email_from: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SteamDetails {
    auth_url: Option<Url>,
    publisher_auth_key: String,
    app_id: u32,
    identify: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LimiterValues {
    /// time until another attempt is allowed
    time_until_another_attempt: Duration,
    /// so many attempts are allowed initially
    initial_request_count: NonZeroU32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LimiterSettings {
    credential_auth_tokens: LimiterValues,
    credential_auth_tokens_secret: LimiterValues,
    account_tokens: LimiterValues,
    account_tokens_secret: LimiterValues,
    login: LimiterValues,
    link_credential: LimiterValues,
    unlink_credential: LimiterValues,
    delete: LimiterValues,
    logout_all: LimiterValues,
    logout: LimiterValues,
    account_info: LimiterValues,
}

impl Default for LimiterSettings {
    fn default() -> Self {
        Self {
            credential_auth_tokens: LimiterValues {
                // once per day
                time_until_another_attempt: Duration::from_secs(60 * 60 * 24),
                // one request total
                initial_request_count: NonZeroU32::new(1).unwrap(),
            },
            credential_auth_tokens_secret: LimiterValues {
                // once per hour
                time_until_another_attempt: Duration::from_secs(60 * 60),
                // 5 request total, since the secret is handled by the
                // account server or related software
                initial_request_count: NonZeroU32::new(5).unwrap(),
            },
            account_tokens: LimiterValues {
                // once per day
                time_until_another_attempt: Duration::from_secs(60 * 60 * 24),
                // one request total
                initial_request_count: NonZeroU32::new(1).unwrap(),
            },
            account_tokens_secret: LimiterValues {
                // once per hour
                time_until_another_attempt: Duration::from_secs(60 * 60),
                // 5 request total, since the secret is handled by the
                // account server or related software
                initial_request_count: NonZeroU32::new(5).unwrap(),
            },
            login: LimiterValues {
                // once per day
                time_until_another_attempt: Duration::from_secs(60 * 60),
                // 5 request total
                initial_request_count: NonZeroU32::new(5).unwrap(),
            },
            link_credential: LimiterValues {
                // once per hour
                time_until_another_attempt: Duration::from_secs(60 * 60),
                // 5 request total
                initial_request_count: NonZeroU32::new(5).unwrap(),
            },
            unlink_credential: LimiterValues {
                // once per hour
                time_until_another_attempt: Duration::from_secs(60 * 60),
                // 5 request total
                initial_request_count: NonZeroU32::new(5).unwrap(),
            },
            delete: LimiterValues {
                // once per hour
                time_until_another_attempt: Duration::from_secs(60 * 60),
                // 5 request total
                initial_request_count: NonZeroU32::new(5).unwrap(),
            },
            logout: LimiterValues {
                // once per hour
                time_until_another_attempt: Duration::from_secs(60 * 60),
                // 5 request total
                initial_request_count: NonZeroU32::new(5).unwrap(),
            },
            logout_all: LimiterValues {
                // once per hour
                time_until_another_attempt: Duration::from_secs(60 * 60),
                // 5 request total
                initial_request_count: NonZeroU32::new(5).unwrap(),
            },
            account_info: LimiterValues {
                // once per minute
                time_until_another_attempt: Duration::from_secs(60),
                // 3 request total
                initial_request_count: NonZeroU32::new(3).unwrap(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Details {
    db: DbDetails,
    http: HttpServerDetails,
    email: EmailDetails,
    steam: SteamDetails,
    limitter: LimiterSettings,
}

pub(crate) async fn prepare_db(details: &DbDetails) -> anyhow::Result<AnyPool> {
    let is_localhost =
        details.host == "localhost" || details.host == "127.0.0.1" || details.host == "::1";

    sqlx::any::install_default_drivers();
    Ok(AnyPool::MySql(
        MySqlPoolOptions::new()
            .max_connections(200)
            .connect_with(
                MySqlConnectOptions::new()
                    .charset("utf8mb4")
                    .host(&details.host)
                    .port(details.port)
                    .database(&details.database)
                    .username(&details.username)
                    .password(&details.password)
                    .ssl_mode(if !is_localhost {
                        sqlx::mysql::MySqlSslMode::Required
                    } else {
                        sqlx::mysql::MySqlSslMode::Preferred
                    })
                    .ssl_ca(&details.ca_cert_path),
            )
            .await?,
    ))
}

pub(crate) async fn prepare_statements(pool: &AnyPool) -> anyhow::Result<DbConnectionShared> {
    let mut connection = pool.acquire().await?;
    let mut connection = connection.acquire().await?;

    // now prepare the statements
    let credential_auth_token_statement = AddCredentialAuthToken::prepare(&mut connection).await?;
    let credential_auth_token_qry_statement =
        CredentialAuthTokenQry::prepare(&mut connection).await?;
    let invalidate_credential_auth_token_statement =
        InvalidateCredentialAuthToken::prepare(&mut connection).await?;
    let try_create_account_statement = TryCreateAccount::prepare(&mut connection).await?;
    let account_id_from_last_insert_qry_statement =
        AccountIdFromLastInsert::prepare(&mut connection).await?;
    let account_id_from_email_qry_statement = AccountIdFromEmail::prepare(&mut connection).await?;
    let account_id_from_steam_qry_statement = AccountIdFromSteam::prepare(&mut connection).await?;
    let link_credentials_email_qry_statement =
        LinkAccountCredentialEmail::prepare(&mut connection).await?;
    let link_credentials_steam_qry_statement =
        LinkAccountCredentialSteam::prepare(&mut connection).await?;
    let create_session_statement = CreateSession::prepare(&mut connection).await?;
    let logout_statement = RemoveSession::prepare(&mut connection).await?;
    let auth_attempt_statement = AuthAttempt::prepare(&mut connection).await?;
    let account_token_email_statement = AddAccountTokenEmail::prepare(&mut connection).await?;
    let account_token_steam_statement = AddAccountTokenSteam::prepare(&mut connection).await?;
    let account_token_qry_statement = AccountTokenQry::prepare(&mut connection).await?;
    let invalidate_account_token_statement =
        InvalidateAccountToken::prepare(&mut connection).await?;
    let remove_sessions_except_statement = RemoveSessionsExcept::prepare(&mut connection).await?;
    let remove_account_statement = RemoveAccount::prepare(&mut connection).await?;
    let add_cert_statement = AddCert::prepare(&mut connection).await?;
    let get_certs_statement = GetCerts::prepare(&mut connection).await?;
    let cleanup_credential_auth_tokens_statement =
        CleanupCredentialAuthTokens::prepare(&mut connection).await?;
    let cleanup_account_tokens_statement = CleanupAccountTokens::prepare(&mut connection).await?;
    let cleanup_certs_statement = CleanupCerts::prepare(&mut connection).await?;
    let unlink_credential_email_statement = UnlinkCredentialEmail::prepare(&mut connection).await?;
    let unlink_credential_steam_statement = UnlinkCredentialSteam::prepare(&mut connection).await?;
    let unlink_credential_by_email_statement =
        UnlinkCredentialByEmail::prepare(&mut connection).await?;
    let unlink_credential_by_steam_statement =
        UnlinkCredentialBySteam::prepare(&mut connection).await?;
    let account_info = AccountInfo::prepare(&mut connection).await?;

    Ok(DbConnectionShared {
        credential_auth_token_statement,
        credential_auth_token_qry_statement,
        invalidate_credential_auth_token_statement,
        try_create_account_statement,
        account_id_from_last_insert_qry_statement,
        account_id_from_email_qry_statement,
        account_id_from_steam_qry_statement,
        link_credentials_email_qry_statement,
        link_credentials_steam_qry_statement,
        create_session_statement,
        logout_statement,
        auth_attempt_statement,
        account_token_email_statement,
        account_token_steam_statement,
        account_token_qry_statement,
        invalidate_account_token_statement,
        remove_sessions_except_statement,
        remove_account_statement,
        add_cert_statement,
        get_certs_statement,
        cleanup_credential_auth_tokens_statement,
        cleanup_account_tokens_statement,
        cleanup_certs_statement,
        unlink_credential_email_statement,
        unlink_credential_steam_statement,
        unlink_credential_by_email_statement,
        unlink_credential_by_steam_statement,
        account_info,
    })
}

pub(crate) async fn prepare_email(details: &EmailDetails) -> anyhow::Result<EmailShared> {
    EmailShared::new(
        &details.relay,
        details.relay_port,
        &details.email_from,
        &details.username,
        &details.password,
    )
    .await
}

pub(crate) fn prepare_steam(details: &SteamDetails) -> anyhow::Result<SteamShared> {
    SteamShared::new(
        details
            .auth_url
            .clone()
            .unwrap_or_else(|| OFFICIAL_STEAM_AUTH_URL.try_into().unwrap()),
        &details.publisher_auth_key,
        details.identify.as_deref(),
        details.app_id,
    )
}

pub(crate) async fn prepare_http(
    details: &HttpServerDetails,
    db: DbConnectionShared,
    email: EmailShared,
    steam: SteamShared,
    pool: &AnyPool,
    settings: &LimiterSettings,
) -> anyhow::Result<(TcpListener, Router, Arc<Shared>)> {
    let keys = tokio::fs::read("signing_keys.json")
        .await
        .map_err(|err| anyhow!(err))
        .and_then(|key| serde_json::from_slice::<PrivateKeys>(&key).map_err(|err| anyhow!(err)));

    let keys = if let Ok(keys) = keys {
        keys
    } else {
        let (key1, cert1) = generate_key_and_cert(true)?;
        store_cert(&db, pool, &cert1).await?;

        let (key2, cert2) = generate_key_and_cert(false)?;
        store_cert(&db, pool, &cert2).await?;

        let res = PrivateKeys {
            current_key: key1,
            current_cert: cert1,
            next_key: key2,
            next_cert: cert2,
        };

        tokio::fs::write("signing_keys.json", serde_json::to_vec(&res)?).await?;

        res
    };

    let certs = get_certs(&db, pool).await?;

    let shared = Arc::new(Shared {
        db,
        email,
        steam,
        ip_ban_list: Arc::new(RwLock::new(IpDenyList::load_from_file().await)),
        signing_keys: Arc::new(RwLock::new(Arc::new(keys))),
        cert_chain: Arc::new(RwLock::new(Arc::new(certs))),
        account_tokens_email: Arc::new(RwLock::new(Arc::new(
            EmailShared::load_email_template("account_tokens.html")
                .await
                .unwrap_or_else(|_| {
                    "<p>Hello %SUBJECT%,</p>\n\
                    <p>Please use the following token to verify your action:</p>\n\
                    <pre>%CODE%</pre>"
                        .to_string()
                }),
        ))),
        credential_auth_tokens_email: Arc::new(RwLock::new(Arc::new(
            EmailShared::load_email_template("credential_auth_tokens.html")
                .await
                .unwrap_or_else(|_| {
                    "<p>Hello %SUBJECT%,</p>\n\
                    <p>Please use the following token to verify your action:</p>\n\
                    <pre>%CODE%</pre>"
                        .to_string()
                }),
        ))),
    });

    // prepare socket
    let tcp_socket = TcpSocket::new_v4()?;
    tcp_socket.set_reuseaddr(true)?;
    tcp_socket.bind(format!("127.0.0.1:{}", details.port).parse()?)?;

    let listener = tcp_socket.listen(1024)?;

    // build http server
    let layer = |limiter: &LimiterValues| {
        anyhow::Ok(
            ServiceBuilder::new().layer(GovernorLayer {
                config: Arc::new(
                    GovernorConfigBuilder::default()
                        .key_extractor(SmartIpKeyExtractor)
                        .period(limiter.time_until_another_attempt)
                        .burst_size(limiter.initial_request_count.get())
                        .error_handler(|err| match err {
                            tower_governor::GovernorError::TooManyRequests { .. } => {
                                Json(AccountServerReqResult::<(), ()>::Err(
                                    AccountServerRequestError::RateLimited(err.to_string()),
                                ))
                                .into_response()
                            }
                            tower_governor::GovernorError::UnableToExtractKey
                            | tower_governor::GovernorError::Other { .. } => {
                                Json(AccountServerReqResult::<(), ()>::Err(
                                    AccountServerRequestError::Other(err.to_string()),
                                ))
                                .into_response()
                            }
                        })
                        .finish()
                        .ok_or_else(|| anyhow!("Could not create governor config."))?,
                ),
            }),
        )
    };

    // Crendential auth tokens
    let shared_clone = shared.clone();
    let pool_clone = pool.clone();
    let token_email = axum::Router::new()
        .route(
            "/email",
            axum::routing::post(move |payload: Json<_>| {
                credential_auth_token_email(shared_clone, pool_clone, false, payload)
            }),
        )
        .layer(layer(&settings.credential_auth_tokens)?);
    let shared_clone = shared.clone();
    let pool_clone = pool.clone();
    let token_steam = axum::Router::new()
        .route(
            "/steam",
            axum::routing::post(move |payload: Json<_>| {
                credential_auth_token_steam(shared_clone, pool_clone, false, payload)
            }),
        )
        .layer(layer(&settings.credential_auth_tokens)?);
    let shared_clone = shared.clone();
    let pool_clone = pool.clone();
    let token_email_secret = axum::Router::new()
        .route(
            "/email-secret",
            axum::routing::post(move |payload: Json<_>| {
                credential_auth_token_email(shared_clone, pool_clone, true, payload)
            }),
        )
        .layer(layer(&settings.credential_auth_tokens_secret)?);
    let shared_clone = shared.clone();
    let pool_clone = pool.clone();
    let token_steam_secret = axum::Router::new()
        .route(
            "/steam-secret",
            axum::routing::post(move |payload: Json<_>| {
                credential_auth_token_steam(shared_clone, pool_clone, true, payload)
            }),
        )
        .layer(layer(&settings.credential_auth_tokens_secret)?);
    let mut app = axum::Router::new();
    app = app.nest(
        "/token",
        Router::new()
            .merge(token_email)
            .merge(token_steam)
            .merge(token_email_secret)
            .merge(token_steam_secret),
    );
    let shared_clone = shared.clone();
    let pool_clone = pool.clone();
    let account_token_by_email = axum::Router::new()
        .route(
            "/email",
            axum::routing::post(move |payload: Json<_>| {
                account_token_email(shared_clone, pool_clone, false, payload)
            }),
        )
        .layer(layer(&settings.account_tokens)?);
    let shared_clone = shared.clone();
    let pool_clone = pool.clone();
    let account_token_secret_email = axum::Router::new()
        .route(
            "/email-secret",
            axum::routing::post(move |payload: Json<_>| {
                account_token_email(shared_clone, pool_clone, true, payload)
            }),
        )
        .layer(layer(&settings.account_tokens_secret)?);
    let shared_clone = shared.clone();
    let pool_clone = pool.clone();
    let account_token_by_steam = axum::Router::new()
        .route(
            "/steam",
            axum::routing::post(move |payload: Json<_>| {
                account_token_steam(shared_clone, pool_clone, false, payload)
            }),
        )
        .layer(layer(&settings.account_tokens)?);
    let shared_clone = shared.clone();
    let pool_clone = pool.clone();
    let account_token_secret_steam = axum::Router::new()
        .route(
            "/steam-secret",
            axum::routing::post(move |payload: Json<_>| {
                account_token_steam(shared_clone, pool_clone, true, payload)
            }),
        )
        .layer(layer(&settings.account_tokens_secret)?);
    app = app.nest(
        "/account-token",
        Router::new()
            .merge(account_token_by_email)
            .merge(account_token_secret_email)
            .merge(account_token_by_steam)
            .merge(account_token_secret_steam),
    );
    // Actual login
    let shared_clone = shared.clone();
    let pool_clone = pool.clone();
    app = app.merge(
        axum::Router::new().route(
            "/login",
            axum::routing::post(move |payload: Json<_>| {
                login_request(shared_clone, pool_clone, payload)
            })
            .layer(layer(&settings.login)?),
        ),
    );
    // Link credential
    let shared_clone = shared.clone();
    let pool_clone = pool.clone();
    app = app.merge(
        axum::Router::new().route(
            "/link-credential",
            axum::routing::post(move |payload: Json<_>| {
                link_credential_request(shared_clone, pool_clone, payload)
            })
            .layer(layer(&settings.link_credential)?),
        ),
    );
    // Unlink credential
    let shared_clone = shared.clone();
    let pool_clone = pool.clone();
    app = app.merge(
        axum::Router::new().route(
            "/unlink-credential",
            axum::routing::post(move |qry: Json<_>| {
                unlink_credential_request(shared_clone, pool_clone, qry)
            })
            .layer(layer(&settings.unlink_credential)?),
        ),
    );
    // Delete account
    let shared_clone = shared.clone();
    let pool_clone = pool.clone();
    app = app.merge(
        axum::Router::new().route(
            "/delete",
            axum::routing::post(move |qry: Json<_>| delete_request(shared_clone, pool_clone, qry))
                .layer(layer(&settings.delete)?),
        ),
    );
    // Logout all
    let shared_clone = shared.clone();
    let pool_clone = pool.clone();
    app = app.merge(
        axum::Router::new().route(
            "/logout-all",
            axum::routing::post(move |qry: Json<_>| {
                logout_all_request(shared_clone, pool_clone, qry)
            })
            .layer(layer(&settings.logout_all)?),
        ),
    );
    // Logout
    let shared_clone = shared.clone();
    let pool_clone = pool.clone();
    app = app.merge(
        axum::Router::new().route(
            "/logout",
            axum::routing::post(move |qry: Json<_>| logout_request(shared_clone, pool_clone, qry))
                .layer(layer(&settings.logout)?),
        ),
    );
    // account info
    let shared_clone = shared.clone();
    let pool_clone = pool.clone();
    app = app.merge(
        axum::Router::new().route(
            "/account-info",
            axum::routing::post(move |qry: Json<_>| {
                account_info_request(shared_clone, pool_clone, qry)
            })
            .layer(layer(&settings.account_info)?),
        ),
    );
    let shared_clone = shared.clone();
    let pool_clone = pool.clone();
    app = app.route(
        "/sign",
        axum::routing::post(move |payload: Json<_>| {
            sign_request(shared_clone, pool_clone, payload)
        }),
    );
    let shared_clone = shared.clone();
    app = app.route(
        "/certs",
        axum::routing::get(move || certs_request(shared_clone)),
    );
    app = app.route("/ping", axum::routing::get(|| async { Json("pong") }));
    // 16 KiB limit should be enough for all requests
    let request_size = DefaultBodyLimit::max(1024 * 16);
    app = app
        .layer(request_size)
        .layer(axum::middleware::from_fn_with_state(
            shared.ip_ban_list.clone(),
            ip_deny_layer,
        ));

    Ok((listener, app, shared))
}

pub(crate) async fn prepare(
    details: &Details,
) -> anyhow::Result<(TcpListener, Router, AnyPool, Arc<Shared>)> {
    // first connect to the database
    let pool = prepare_db(&details.db).await?;

    let db = prepare_statements(&pool).await?;
    let email = prepare_email(&details.email).await?;
    let steam = prepare_steam(&details.steam)?;
    let (listener, app, shared) =
        prepare_http(&details.http, db, email, steam, &pool, &details.limitter).await?;

    Ok((listener, app, pool, shared))
}

/// Returns the time the impl should wait before calling this function again.
/// Returns `Either::Right` if a new cert was created, else `Either::Left`.
pub(crate) async fn generate_new_signing_keys_impl(
    pool: &AnyPool,
    shared: &Arc<Shared>,
    now: SystemTime,
    default_check_key_time: Duration,
    err_check_key_time: Duration,
    validy_extra_offset: Duration,
) -> Either<Duration, Duration> {
    // once per day check if a new signing key should be created
    let mut next_sleep_time = Either::Left(default_check_key_time);
    let err_check_key_time = Either::Left(err_check_key_time);

    let check_keys = shared.signing_keys.read().clone();
    if now + validy_extra_offset
        >= check_keys
            .current_cert
            .tbs_certificate
            .validity
            .not_after
            .to_system_time()
    {
        // create a new key & cert, switch next key to current
        if let Ok((key, cert)) = generate_key_and_cert(false) {
            let store_res = store_cert(&shared.db, pool, &cert).await;
            if store_res.is_err() {
                next_sleep_time = err_check_key_time;
            } else if let Ok(certs) = get_certs(&shared.db, pool).await {
                let cur_keys = shared.signing_keys.read().clone();
                let new_keys = Arc::new(PrivateKeys {
                    current_key: cur_keys.next_key.clone(),
                    current_cert: cur_keys.next_cert.clone(),
                    next_key: key,
                    next_cert: cert,
                });
                if let Ok(val) = serde_json::to_vec(new_keys.as_ref()) {
                    if tokio::fs::write("signing_keys.json", val).await.is_ok() {
                        *shared.cert_chain.write() = Arc::new(certs);
                        *shared.signing_keys.write() = new_keys;
                        next_sleep_time = Either::Right(default_check_key_time);
                    } else {
                        next_sleep_time = err_check_key_time;
                    }
                } else {
                    next_sleep_time = err_check_key_time;
                }
            } else {
                next_sleep_time = err_check_key_time;
            }
        } else {
            next_sleep_time = err_check_key_time;
        }
    }
    next_sleep_time
}

pub(crate) async fn generate_new_signing_keys(pool: &AnyPool, shared: &Arc<Shared>) -> Duration {
    match generate_new_signing_keys_impl(
        pool,
        shared,
        SystemTime::now(),
        Duration::from_secs(60 * 60 * 24),
        Duration::from_secs(60 * 60 * 2),
        Duration::from_secs(60 * 60 * 24 * 7),
    )
    .await
    {
        Either::Left(r) | Either::Right(r) => r,
    }
}

async fn regenerate_signing_keys_and_certs(pool: AnyPool, shared: Arc<Shared>) -> ! {
    loop {
        let next_sleep_time = generate_new_signing_keys(&pool, &shared).await;

        tokio::time::sleep(next_sleep_time).await;

        // get latest certs
        if let Ok(certs) = get_certs(&shared.db, &pool).await {
            *shared.cert_chain.write() = Arc::new(certs);
        }
    }
}

// https://github.com/tokio-rs/tokio/issues/5616
#[allow(clippy::redundant_pub_crate)]
pub(crate) async fn run(
    listener: TcpListener,
    app: Router,
    pool: AnyPool,
    shared: Arc<Shared>,
    handle_updates: bool,
) -> anyhow::Result<()> {
    let pool_clone = pool.clone();
    let shared_clone = shared.clone();
    let shared_watchers = shared.clone();
    let app = app.into_make_service_with_connect_info::<SocketAddr>();
    tokio::select!(
        err = async move { axum::serve(listener, app).await } => {
           err?;
        },
        _ = async move {
            regenerate_signing_keys_and_certs(pool, shared).await
        }, if handle_updates => {}
        _ = async move {
            update(pool_clone, shared_clone).await
        }, if handle_updates => {}
        _ = async move {
            handle_watchers(shared_watchers).await
        }, if handle_updates => {}
    );
    Ok(())
}

#[tokio::main]
async fn main() {
    if std::env::var("RUST_LOG").is_err() {
        // rust nightly compatibility
        #[allow(unused_unsafe)]
        unsafe {
            std::env::set_var("RUST_LOG", "info")
        };
    }
    env_logger::init();

    let mut cmd = command!()
        .about("The account server using http & mysql.")
        .arg(
            Arg::new("setup")
                .long("setup")
                .help("Setup the account server, e.g. fill the mysql tables.")
                .required(false)
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("cleanup")
                .long("cleanup")
                .help("Cleanup the account server, e.g. remove the mysql tables.")
                .required(false)
                .action(ArgAction::SetTrue),
        );
    cmd.build();
    let m = cmd.get_matches();

    let print_settings_err = || {
        log::error!(
            "a settings.json looks like this\n{}",
            serde_json::to_string_pretty(&Details {
                db: DbDetails {
                    host: "localhost".to_string(),
                    port: 3306,
                    database: "ddnet_accounts".to_string(),
                    username: "user".to_string(),
                    password: "password".to_string(),
                    ca_cert_path: "/etc/mysql/ssl/ca-cert.pem".into()
                },
                http: HttpServerDetails { port: 443 },
                email: EmailDetails {
                    relay: "emails.localhost".to_string(),
                    relay_port: 465,
                    username: "account".to_string(),
                    password: "email-password".to_string(),
                    email_from: "account@localhost".to_string(),
                },
                steam: SteamDetails {
                    auth_url: None,
                    publisher_auth_key: "publisher_auth_key".into(),
                    app_id: 123,
                    identify: None
                },
                limitter: Default::default()
            })
            .unwrap()
        )
    };

    let Ok(cfg) = tokio::fs::read("settings.json").await else {
        log::error!("no settings.json found, please create one.");
        print_settings_err();

        panic!("failed to find settings.json, see log for more information");
    };

    let Ok(details) = serde_json::from_slice::<Details>(&cfg) else {
        log::error!("settings.json was invalid.");
        print_settings_err();

        panic!("settings were not a valid json file, see log for more information");
    };

    if m.value_source("setup")
        .is_some_and(|s| matches!(s, ValueSource::CommandLine))
    {
        let pool = prepare_db(&details.db).await.unwrap();
        setup::setup(&pool).await.unwrap();
    } else if m
        .value_source("cleanup")
        .is_some_and(|s| matches!(s, ValueSource::CommandLine))
    {
        let pool = prepare_db(&details.db).await.unwrap();
        setup::delete(&pool).await.unwrap();
    } else {
        let (listener, app, pool, shared) = prepare(&details).await.unwrap();
        run(listener, app, pool, shared, true).await.unwrap();
    }
}
