use std::fmt::Debug;

use async_trait::async_trait;
use ddnet_account_client::errors::HttpLikeError;
use url::Url;

#[async_trait]
pub trait Http: Debug + Sync + Send {
    fn new(base_url: Url) -> Self
    where
        Self: Sized;
    async fn post_json(&self, url: Url, data: Vec<u8>) -> anyhow::Result<Vec<u8>, HttpLikeError>;
    async fn get(&self, url: Url) -> anyhow::Result<Vec<u8>, HttpLikeError>;
    fn base_url(&self) -> Url;
}
