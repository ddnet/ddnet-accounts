use std::{ops::Deref, path::Path, sync::Arc};

use async_trait::async_trait;
use ddnet_account_client::{
    errors::{FsLikeError, HttpLikeError},
    interface::Io,
};
use ddnet_account_client_http_fs::{client::ClientHttpTokioFs, fs::Fs, http::Http};
use reqwest::header::{HeaderValue, CONTENT_TYPE};
use url::Url;

#[derive(Debug)]
pub struct HttpReqwest {
    base_url: Url,
    http: reqwest::Client,
}

#[async_trait]
impl Http for HttpReqwest {
    fn new(base_url: Url) -> Self
    where
        Self: Sized,
    {
        Self {
            base_url,
            http: reqwest::ClientBuilder::new().build().unwrap(),
        }
    }
    async fn post_json(&self, url: Url, data: Vec<u8>) -> anyhow::Result<Vec<u8>, HttpLikeError> {
        let res = self
            .http
            .post(url)
            .header(CONTENT_TYPE, HeaderValue::from_static("application/json"))
            .body(data)
            .send()
            .await
            .map_err(|err| {
                if err.is_request() {
                    HttpLikeError::Request
                } else if err.is_status() {
                    HttpLikeError::Status(err.status().unwrap().as_u16())
                } else {
                    HttpLikeError::Other(err.into())
                }
            })?;
        Ok(res
            .bytes()
            .await
            .map_err(|err| HttpLikeError::Other(err.into()))?
            .to_vec())
    }
    async fn get(&self, url: Url) -> anyhow::Result<Vec<u8>, HttpLikeError> {
        let res = self.http.get(url).send().await.map_err(|err| {
            if err.is_request() {
                HttpLikeError::Request
            } else if err.is_status() {
                HttpLikeError::Status(err.status().unwrap().as_u16())
            } else {
                HttpLikeError::Other(err.into())
            }
        })?;
        Ok(res
            .bytes()
            .await
            .map_err(|err| HttpLikeError::Other(err.into()))?
            .to_vec())
    }
    fn base_url(&self) -> Url {
        self.base_url.clone()
    }
}

#[derive(Debug)]
pub struct ClientReqwestTokioFs {
    pub client: Arc<ClientHttpTokioFs>,
}

impl ClientReqwestTokioFs {
    pub async fn new(base_urls: Vec<Url>, secure_path: &Path) -> anyhow::Result<Self, FsLikeError> {
        Ok(Self {
            client: Arc::new(ClientHttpTokioFs {
                http: base_urls
                    .into_iter()
                    .map(|base_url| {
                        let res: Arc<dyn Http> = Arc::new(HttpReqwest::new(base_url));
                        res
                    })
                    .collect(),
                cur_http: Default::default(),
                fs: Fs::new(secure_path.into()).await?,
            }),
        })
    }
}

impl Deref for ClientReqwestTokioFs {
    type Target = dyn Io;

    fn deref(&self) -> &Self::Target {
        self.client.as_ref()
    }
}
