use std::{num::NonZeroU32, sync::Arc, time::Duration};

use axum::{extract::Query, response::IntoResponse, routing::get, Router};
use lettre::SmtpTransport;
use parking_lot::Mutex;
use serde::Deserialize;
use sqlx::{Any, Pool};
use tokio::{net::TcpSocket, task::JoinHandle};

use crate::{
    email::{EmailHook, EmailShared},
    prepare_db, prepare_http, prepare_statements, run, setup,
    shared::Shared,
    steam::{self, SteamHook, SteamShared},
};

pub async fn test_setup() -> anyhow::Result<Pool<Any>> {
    prepare_db(&crate::DbDetails {
        host: "localhost".into(),
        port: 3306,
        database: "ddnet_account_test".into(),
        username: "ddnet-account-test".into(),
        password: "test".into(),
        ca_cert_path: "/etc/mysql/ssl/ca-cert.pem".into(),
    })
    .await
}

pub struct TestAccServer {
    pub(crate) server: JoinHandle<anyhow::Result<()>>,
    pub(crate) pool: Pool<Any>,
    pub(crate) shared: Arc<Shared>,
    pub(crate) steam: JoinHandle<anyhow::Result<()>>,
}

impl TestAccServer {
    pub(crate) async fn new(
        token: Arc<Mutex<String>>,
        account_token: Arc<Mutex<String>>,
        limit: bool,
        email_test_mode: bool,
    ) -> anyhow::Result<Self> {
        let pool = test_setup().await?;

        if let Err(err) = setup::delete(&pool).await {
            println!("warning: {}", err);
        }
        setup::setup(&pool).await?;

        let db = prepare_statements(&pool).await?;
        let mut email: EmailShared =
            ("test@localhost", SmtpTransport::unencrypted_localhost()).into();
        email.set_test_mode(email_test_mode);
        #[derive(Debug)]
        struct EmailReader {
            token: Arc<Mutex<String>>,
            account_token: Arc<Mutex<String>>,
        }
        impl EmailHook for EmailReader {
            fn on_mail(&self, email_subject: &str, email_body: &str) {
                if [
                    "DDNet Logout All Sessions",
                    "DDNet Link Credential",
                    "DDNet Delete Account",
                ]
                .contains(&email_subject)
                {
                    let reg = regex::Regex::new(r".*<pre>(.*)</pre>.*").unwrap();
                    let (_, [account_token]): (&str, [&str; 1]) =
                        reg.captures_iter(email_body).next().unwrap().extract();
                    dbg!(account_token);
                    *self.account_token.lock() = account_token.to_string();
                } else {
                    let reg = regex::Regex::new(r".*<pre>(.*)</pre>.*").unwrap();
                    let (_, [token]): (&str, [&str; 1]) =
                        reg.captures_iter(email_body).next().unwrap().extract();
                    dbg!(token);
                    *self.token.lock() = token.to_string();
                }
            }
        }
        email.set_hook(EmailReader {
            token: token.clone(),
            account_token: account_token.clone(),
        });

        let mut steam = SteamShared::new(
            "http://127.0.0.1:3344".try_into()?,
            "my_secret_pub_auth_key",
            Some("account"),
            123,
        )?;
        #[derive(Debug)]
        struct SteamReader {}
        impl SteamHook for SteamReader {
            fn on_steam_code(&self, steam_code: &[u8]) {
                dbg!(steam_code);
            }
        }
        steam.set_hook(SteamReader {});

        // create a fake steam server
        let tcp_socket = TcpSocket::new_v4()?;
        tcp_socket.set_reuseaddr(true)?;
        tcp_socket.bind(format!("127.0.0.1:{}", 3344).parse()?)?;

        // from https://partner.steamgames.com/doc/webapi/ISteamUserAuth#AuthenticateUserTicket
        #[derive(Debug, Deserialize)]
        struct SteamQueryParams {
            pub key: String,
            pub appid: u32,
            pub ticket: String,
            pub identity: Option<String>,
        }
        async fn steam_id_check(
            Query(q): Query<SteamQueryParams>,
        ) -> axum::response::Response<axum::body::Body> {
            assert!(q.ticket == hex::encode("justatest"));
            dbg!(q.key, q.appid, q.ticket, q.identity);
            axum::Json(steam::HttpResult {
                response: steam::TicketAuthResponse {
                    params: steam::SteamUser {
                        result: "".to_string(),
                        steam_id: 0.to_string(),
                        owner_steam_id: 0.to_string(),
                        vac_banned: false,
                        publisher_banned: false,
                    },
                },
            })
            .into_response()
        }
        let app = Router::new().route("/", get(steam_id_check));

        let listener = tcp_socket.listen(1024)?;
        let steam_handle =
            tokio::spawn(async move { anyhow::Ok(axum::serve(listener, app).await?) });

        let limit = if limit {
            crate::LimiterSettings::default()
        } else {
            crate::LimiterSettings {
                credential_auth_tokens: crate::LimiterValues {
                    time_until_another_attempt: Duration::from_nanos(1),
                    initial_request_count: NonZeroU32::new(u32::MAX).unwrap(),
                },
                credential_auth_tokens_secret: crate::LimiterValues {
                    time_until_another_attempt: Duration::from_nanos(1),
                    initial_request_count: NonZeroU32::new(u32::MAX).unwrap(),
                },
                account_tokens: crate::LimiterValues {
                    time_until_another_attempt: Duration::from_nanos(1),
                    initial_request_count: NonZeroU32::new(u32::MAX).unwrap(),
                },
                account_tokens_secret: crate::LimiterValues {
                    time_until_another_attempt: Duration::from_nanos(1),
                    initial_request_count: NonZeroU32::new(u32::MAX).unwrap(),
                },
                login: crate::LimiterValues {
                    time_until_another_attempt: Duration::from_nanos(1),
                    initial_request_count: NonZeroU32::new(u32::MAX).unwrap(),
                },
                link_credential: crate::LimiterValues {
                    time_until_another_attempt: Duration::from_nanos(1),
                    initial_request_count: NonZeroU32::new(u32::MAX).unwrap(),
                },
                unlink_credential: crate::LimiterValues {
                    time_until_another_attempt: Duration::from_nanos(1),
                    initial_request_count: NonZeroU32::new(u32::MAX).unwrap(),
                },
                delete: crate::LimiterValues {
                    time_until_another_attempt: Duration::from_nanos(1),
                    initial_request_count: NonZeroU32::new(u32::MAX).unwrap(),
                },
                logout_all: crate::LimiterValues {
                    time_until_another_attempt: Duration::from_nanos(1),
                    initial_request_count: NonZeroU32::new(u32::MAX).unwrap(),
                },
                logout: crate::LimiterValues {
                    time_until_another_attempt: Duration::from_nanos(1),
                    initial_request_count: NonZeroU32::new(u32::MAX).unwrap(),
                },
                account_info: crate::LimiterValues {
                    time_until_another_attempt: Duration::from_nanos(1),
                    initial_request_count: NonZeroU32::new(u32::MAX).unwrap(),
                },
            }
        };
        let (listener, app, shared) = prepare_http(
            &crate::HttpServerDetails { port: 4433 },
            db,
            email,
            steam,
            &pool,
            &limit,
        )
        .await?;

        let pool_clone = pool.clone();
        let shared_clone = shared.clone();
        let server =
            tokio::spawn(async move { run(listener, app, pool_clone, shared_clone, false).await });

        Ok(Self {
            server,
            pool,
            shared,
            steam: steam_handle,
        })
    }

    pub(crate) async fn destroy(self) -> anyhow::Result<()> {
        self.server.abort();
        self.steam.abort();

        let _ = self.server.await;

        setup::delete(&self.pool).await?;
        anyhow::Ok(())
    }
}

pub struct TestGameServer {
    pool: Pool<Any>,
    pub(crate) game_server_data: Arc<ddnet_account_game_server::shared::Shared>,
}

impl TestGameServer {
    pub(crate) async fn new(pool: &Pool<Any>) -> anyhow::Result<Self> {
        // make sure the tables are gone
        let _ = ddnet_account_game_server::setup::delete(pool).await;
        ddnet_account_game_server::setup::setup(pool).await?;

        let game_server_data = ddnet_account_game_server::prepare::prepare(pool).await?;

        Ok(Self {
            pool: pool.clone(),
            game_server_data,
        })
    }

    pub(crate) async fn destroy(self) -> anyhow::Result<()> {
        ddnet_account_game_server::setup::delete(&self.pool).await?;
        Ok(())
    }
}
