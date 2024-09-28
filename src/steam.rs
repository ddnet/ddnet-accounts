use std::{fmt::Debug, sync::Arc};

use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SteamUser {
    pub result: String,
    #[serde(rename = "steamid")]
    pub steam_id: String,
    #[serde(rename = "ownersteamid")]
    pub owner_steam_id: String,
    #[serde(rename = "vacbanned")]
    pub vac_banned: bool,
    #[serde(rename = "publisherbanned")]
    pub publisher_banned: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TicketAuthResponse {
    pub params: SteamUser,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HttpResult {
    pub response: TicketAuthResponse,
}

pub trait SteamHook: Debug + Sync + Send {
    fn on_steam_code(&self, steam_code: &[u8]);
}

#[derive(Debug)]
struct SteamHookDummy {}
impl SteamHook for SteamHookDummy {
    fn on_steam_code(&self, _steam_code: &[u8]) {
        // empty
    }
}

/// Shared steam helper
#[derive(Debug)]
pub struct SteamShared {
    http: reqwest::Client,
    steam_hook: Arc<dyn SteamHook>,

    steam_auth_url: String,
    publisher_auth_key: String,
    identity: Option<String>,
    app_id: u32,
}

/// https://partner.steamgames.com/doc/webapi/ISteamUserAuth#AuthenticateUserTicket
pub const OFFICIAL_STEAM_AUTH_URL: &str =
    "https://partner.steam-api.com/ISteamUserAuth/AuthenticateUserTicket/v1/";

impl SteamShared {
    pub fn new(
        steam_auth_url: Url,
        publisher_auth_key: &str,
        identity: Option<&str>,
        app_id: u32,
    ) -> anyhow::Result<Self> {
        let http = reqwest::Client::new();

        Ok(Self {
            http,
            steam_hook: Arc::new(SteamHookDummy {}),

            app_id,
            publisher_auth_key: publisher_auth_key.to_string(),
            identity: identity.map(|i| i.to_string()),
            steam_auth_url: steam_auth_url.to_string(),
        })
    }

    /// A hook that can see all sent steam token requests.
    /// Currently only useful for testing
    #[allow(dead_code)]
    pub fn set_hook<F: SteamHook + 'static>(&mut self, hook: F) {
        self.steam_hook = Arc::new(hook);
    }

    pub async fn verify_steamid64(&self, steam_ticket: Vec<u8>) -> anyhow::Result<i64> {
        self.steam_hook.on_steam_code(&steam_ticket);

        let ticket = hex::encode(steam_ticket);

        let url = self.identity.as_ref().map_or_else(
            || {
                format!(
                    "{}?key={}&appid={}&ticket={}",
                    self.steam_auth_url, self.publisher_auth_key, self.app_id, ticket
                )
            },
            |identity| {
                format!(
                    "{}?key={}&appid={}&ticket={}&identity={}",
                    self.steam_auth_url, self.publisher_auth_key, self.app_id, ticket, identity
                )
            },
        );

        let steam_ticket_res: String = self.http.get(url).send().await?.text().await?;

        let ticket_res: HttpResult = serde_json::from_str(&steam_ticket_res)?;
        Ok(ticket_res.response.params.steam_id.parse()?)
    }
}

#[cfg(test)]
mod test {
    use axum::{extract::Query, response::IntoResponse, routing::get, Json, Router};
    use serde::Deserialize;

    use crate::steam::{HttpResult, SteamShared};

    #[tokio::test]
    async fn steam_test() {
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
            dbg!(q.key, q.appid, q.ticket, q.identity);

            Json(HttpResult {
                response: crate::steam::TicketAuthResponse {
                    params: crate::steam::SteamUser {
                        result: "Ok".to_string(),
                        steam_id: "0".to_string(),
                        owner_steam_id: "0".to_string(),
                        vac_banned: false,
                        publisher_banned: false,
                    },
                },
            })
            .into_response()
        }
        let app = Router::new().route("/", get(steam_id_check));

        let listener = tokio::net::TcpListener::bind("127.0.0.1:4433")
            .await
            .unwrap();
        tokio::spawn(async move { axum::serve(listener, app).await });

        let steam = SteamShared::new(
            "http://127.0.0.1:4433/".try_into().unwrap(),
            "the_secret_publisher_key",
            Some("account"),
            1337,
        )
        .unwrap();

        let steamid = steam.verify_steamid64(vec![]).await.unwrap();
        assert!(steamid == 0);
    }
}
