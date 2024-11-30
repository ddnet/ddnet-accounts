use std::{
    sync::{atomic::AtomicUsize, Arc},
    time::Duration,
};

use anyhow::anyhow;
use ddnet_account_client::{
    errors::{FsLikeError, HttpLikeError},
    interface::Io,
};
use serde::{Deserialize, Serialize};

use crate::{fs::Fs, http::Http};

#[derive(Debug, Serialize, Deserialize)]
struct FastestHttp {
    index: u64,
    valid_until: chrono::DateTime<chrono::Utc>,
}

/// An extension to the client for deleting the current
/// directory.
#[async_trait::async_trait]
pub trait DeleteAccountExt: Sync + Send {
    async fn remove_account(&self) -> anyhow::Result<(), FsLikeError>;
}

#[derive(Debug)]
pub struct ClientHttpTokioFs {
    pub http: Vec<Arc<dyn Http>>,
    pub cur_http: AtomicUsize,
    pub fs: Fs,
}

impl ClientHttpTokioFs {
    async fn post_json_impl(
        &self,
        http_index: usize,
        url: &str,
        data: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>, HttpLikeError> {
        let http = &self.http[http_index];
        http.post_json(
            http.base_url()
                .join(url)
                .map_err(|err| HttpLikeError::Other(err.into()))?,
            data,
        )
        .await
    }

    async fn backup_post_json(
        &self,
        except_http_index: usize,
        url: &str,
        data: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>, HttpLikeError> {
        for i in 0..self.http.len() {
            if i == except_http_index {
                continue;
            }
            match self.post_json_impl(i, url, data.clone()).await {
                Ok(res) => {
                    self.cur_http.store(i, std::sync::atomic::Ordering::Relaxed);
                    return Ok(res);
                }
                Err(err) => match err {
                    HttpLikeError::Request | HttpLikeError::Status(_) => {
                        // try another http instance
                    }
                    HttpLikeError::Other(err) => {
                        return Err(HttpLikeError::Other(err));
                    }
                },
            }
        }
        Err(HttpLikeError::Request)
    }

    pub async fn post_json(
        &self,
        url: &str,
        data: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>, HttpLikeError> {
        let http_index = self.cur_http.load(std::sync::atomic::Ordering::Relaxed);
        match self.post_json_impl(http_index, url, data.clone()).await {
            Ok(res) => Ok(res),
            Err(err) => match err {
                HttpLikeError::Request | HttpLikeError::Status(_) => {
                    match self.backup_post_json(http_index, url, data).await {
                        Ok(data) => Ok(data),
                        Err(_) => Err(err),
                    }
                }
                HttpLikeError::Other(err) => Err(HttpLikeError::Other(err)),
            },
        }
    }

    async fn get_json_http(
        http: &Arc<dyn Http>,
        url: &str,
    ) -> anyhow::Result<Vec<u8>, HttpLikeError> {
        http.get(
            http.base_url()
                .join(url)
                .map_err(|err| HttpLikeError::Other(err.into()))?,
        )
        .await
    }

    async fn get_json_impl(
        &self,
        http_index: usize,
        url: &str,
    ) -> anyhow::Result<Vec<u8>, HttpLikeError> {
        let http = &self.http[http_index];
        Self::get_json_http(http, url).await
    }

    async fn backup_get_json(
        &self,
        except_http_index: usize,
        url: &str,
    ) -> anyhow::Result<Vec<u8>, HttpLikeError> {
        for i in 0..self.http.len() {
            if i == except_http_index {
                continue;
            }
            match self.get_json_impl(i, url).await {
                Ok(res) => {
                    self.cur_http.store(i, std::sync::atomic::Ordering::Relaxed);
                    return Ok(res);
                }
                Err(err) => match err {
                    HttpLikeError::Request | HttpLikeError::Status(_) => {
                        // try another http instance
                    }
                    HttpLikeError::Other(err) => {
                        return Err(HttpLikeError::Other(err));
                    }
                },
            }
        }
        Err(HttpLikeError::Request)
    }

    pub async fn get_json(&self, url: &str) -> anyhow::Result<Vec<u8>, HttpLikeError> {
        let http_index = self.cur_http.load(std::sync::atomic::Ordering::Relaxed);
        match self.get_json_impl(http_index, url).await {
            Ok(res) => Ok(res),
            Err(err) => match err {
                HttpLikeError::Request | HttpLikeError::Status(_) => {
                    match self.backup_get_json(http_index, url).await {
                        Ok(data) => Ok(data),
                        Err(_) => Err(err),
                    }
                }
                HttpLikeError::Other(err) => Err(HttpLikeError::Other(err)),
            },
        }
    }

    async fn evalulate_fastest_http(http: &[Arc<dyn Http>]) -> usize {
        let mut handles: Vec<_> = Default::default();
        for (i, http) in http.iter().enumerate() {
            let http = http.clone();
            handles.push((
                tokio::spawn(async move {
                    let i = std::time::Instant::now();
                    match Self::get_json_http(&http, "/ping").await {
                        Ok(_) => Some(std::time::Instant::now().saturating_duration_since(i)),
                        Err(_) => None,
                    }
                }),
                i,
            ));
        }
        let mut results: Vec<_> = Default::default();
        for (task, i) in handles {
            if let Ok(Some(time)) = task.await {
                results.push((time, i));
            }
        }
        results
            .into_iter()
            .min_by_key(|(time, _)| *time)
            .map(|(_, index)| index)
            .unwrap_or_default()
    }

    pub async fn get_fastest_http(fs: &Fs, http: &[Arc<dyn Http>]) -> usize {
        let eval_fastest = || {
            Box::pin(async {
                let index = Self::evalulate_fastest_http(http).await;
                let _ = fs
                    .write(
                        "".as_ref(),
                        "fastest_http.json".as_ref(),
                        serde_json::to_vec(&FastestHttp {
                            index: index as u64,
                            valid_until: chrono::Utc::now()
                                + Duration::from_secs(60 * 60 * 24 * 30),
                        })
                        .unwrap(),
                    )
                    .await;
                index
            })
        };
        match fs
            .read("fastest_http.json".as_ref())
            .await
            .map_err(|err| anyhow!(err))
            .and_then(|json| {
                serde_json::from_slice::<FastestHttp>(&json).map_err(|err| anyhow!(err))
            })
            .and_then(|fastest_http| {
                if chrono::Utc::now() < fastest_http.valid_until {
                    Ok(fastest_http)
                } else {
                    Err(anyhow!("fastest_http not valid any more."))
                }
            }) {
            Ok(fastest_http) => {
                if (fastest_http.index as usize) < http.len() {
                    fastest_http.index as usize
                } else {
                    eval_fastest().await
                }
            }
            Err(_) => eval_fastest().await,
        }
    }
}

#[async_trait::async_trait]
impl Io for ClientHttpTokioFs {
    async fn request_credential_auth_email_token(
        &self,
        data: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>, HttpLikeError> {
        self.post_json("/token/email", data).await
    }
    async fn request_credential_auth_steam_token(
        &self,
        data: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>, HttpLikeError> {
        self.post_json("/token/steam", data).await
    }
    async fn request_credential_auth_email_token_with_secret_key(
        &self,
        data: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>, HttpLikeError> {
        self.post_json("/token/email-secret", data).await
    }
    async fn request_credential_auth_steam_token_with_secret_key(
        &self,
        data: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>, HttpLikeError> {
        self.post_json("/token/steam-secret", data).await
    }
    async fn request_login(&self, data: Vec<u8>) -> anyhow::Result<Vec<u8>, HttpLikeError> {
        self.post_json("/login", data).await
    }
    async fn request_logout(&self, data: Vec<u8>) -> anyhow::Result<Vec<u8>, HttpLikeError> {
        self.post_json("/logout", data).await
    }
    async fn request_sign(&self, data: Vec<u8>) -> anyhow::Result<Vec<u8>, HttpLikeError> {
        self.post_json("/sign", data).await
    }
    async fn request_account_token_email(
        &self,
        data: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>, HttpLikeError> {
        self.post_json("/account-token/email", data).await
    }
    async fn request_account_token_email_secret(
        &self,
        data: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>, HttpLikeError> {
        self.post_json("/account-token/email-secret", data).await
    }
    async fn request_account_token_steam(
        &self,
        data: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>, HttpLikeError> {
        self.post_json("/account-token/steam", data).await
    }
    async fn request_account_token_steam_secret(
        &self,
        data: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>, HttpLikeError> {
        self.post_json("/account-token/steam-secret", data).await
    }
    async fn request_logout_all(&self, data: Vec<u8>) -> anyhow::Result<Vec<u8>, HttpLikeError> {
        self.post_json("/logout-all", data).await
    }
    async fn request_delete_account(
        &self,
        data: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>, HttpLikeError> {
        self.post_json("/delete", data).await
    }
    async fn request_link_credential(
        &self,
        data: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>, HttpLikeError> {
        self.post_json("/link-credential", data).await
    }
    async fn request_unlink_credential(
        &self,
        data: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>, HttpLikeError> {
        self.post_json("/unlink-credential", data).await
    }
    async fn request_account_info(&self, data: Vec<u8>) -> anyhow::Result<Vec<u8>, HttpLikeError> {
        self.post_json("/account-info", data).await
    }
    async fn download_account_server_certificates(&self) -> anyhow::Result<Vec<u8>, HttpLikeError> {
        self.get_json("/certs").await
    }
    async fn write_serialized_session_key_pair(
        &self,
        file: Vec<u8>,
    ) -> anyhow::Result<(), FsLikeError> {
        self.fs
            .write("".as_ref(), "account.key".as_ref(), file)
            .await
    }
    async fn read_serialized_session_key_pair(&self) -> anyhow::Result<Vec<u8>, FsLikeError> {
        self.fs.read("account.key".as_ref()).await
    }
    async fn remove_serialized_session_key_pair(&self) -> anyhow::Result<(), FsLikeError> {
        self.fs.remove("account.key".as_ref()).await
    }
}

#[async_trait::async_trait]
impl DeleteAccountExt for ClientHttpTokioFs {
    async fn remove_account(&self) -> anyhow::Result<(), FsLikeError> {
        self.fs.delete().await
    }
}
