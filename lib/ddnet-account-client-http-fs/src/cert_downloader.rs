use std::{
    sync::{Arc, RwLock},
    time::{Duration, SystemTime},
};

use anyhow::anyhow;
use chrono::{DateTime, TimeDelta, Utc};
use ddnet_account_client::certs::certs_to_pub_keys;
use ddnet_accounts_shared::game_server::user_id::VerifyingKey;
use serde::{Deserialize, Serialize};
use tokio::time::Instant;
use x509_cert::der::{Decode, Encode};

use crate::{client::ClientHttpTokioFs, fs::Fs};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CertsDownloaderProps {
    certs_der: Vec<Vec<u8>>,
    last_request: DateTime<Utc>,
}

/// Helper to download the latest public certificates
/// of the account server(s).
///
/// Automatically redownloads certificates if
/// the current ones are about to expire.
#[derive(Debug)]
pub struct CertsDownloader {
    client: Arc<ClientHttpTokioFs>,
    account_server_public_keys: RwLock<Arc<Vec<VerifyingKey>>>,
    cur_certs: RwLock<Vec<x509_cert::Certificate>>,
    last_request: RwLock<DateTime<Utc>>,
}

impl CertsDownloader {
    pub async fn new(client: Arc<ClientHttpTokioFs>) -> anyhow::Result<Arc<Self>> {
        // try to read the key from disk
        let certs_file = client
            .fs
            .read("account_server_certs.json".as_ref())
            .await
            .map_err(|err| anyhow!(err))
            .and_then(|cert_json| {
                serde_json::from_slice::<CertsDownloaderProps>(&cert_json)
                    .map_err(|err| anyhow!(err))
                    .and_then(|props| {
                        props
                            .certs_der
                            .into_iter()
                            .map(|cert_der| {
                                x509_cert::Certificate::from_der(&cert_der)
                                    .map_err(|err| anyhow!(err))
                            })
                            .collect::<anyhow::Result<Vec<x509_cert::Certificate>>>()
                            .map(|certs| (certs, props.last_request))
                    })
            });

        match certs_file {
            Ok((certs_file, last_request)) => Ok(Arc::new(Self {
                client,
                account_server_public_keys: RwLock::new(Arc::new(certs_to_pub_keys(&certs_file))),
                cur_certs: RwLock::new(certs_file),
                last_request: RwLock::new(last_request),
            })),
            Err(_) => {
                // try to download latest cert instead
                let certs = ddnet_account_client::certs::download_certs(client.as_ref()).await?;

                let now_utc = Utc::now();
                let _ = Self::write_file(&client.fs, &certs, now_utc).await;

                Ok(Arc::new(Self {
                    account_server_public_keys: RwLock::new(Arc::new(certs_to_pub_keys(&certs))),
                    client,
                    cur_certs: RwLock::new(certs),
                    last_request: RwLock::new(now_utc),
                }))
            }
        }
    }

    async fn write_file(
        fs: &Fs,
        certs: &[x509_cert::Certificate],
        last_request: DateTime<Utc>,
    ) -> anyhow::Result<()> {
        Ok(fs
            .write(
                "".as_ref(),
                "account_server_certs.json".as_ref(),
                serde_json::to_vec(&CertsDownloaderProps {
                    certs_der: certs
                        .iter()
                        .map(|cert| cert.to_der().map_err(|err| anyhow!(err)))
                        .collect::<anyhow::Result<Vec<_>>>()?,
                    last_request,
                })?,
            )
            .await?)
    }

    /// Returns the duration when the next certificate gets invalid,
    /// or `None` if no certificate exists.
    ///
    /// `now_offset` gives `now` an additional offset to make
    /// the calculation more robust against inaccurate sleeps,
    /// or system time out of syncs.
    /// (Should be around at least 1 day).
    pub fn invalid_in(&self, now: SystemTime, now_offset: Duration) -> Option<Duration> {
        self.cur_certs
            .read()
            .unwrap()
            .iter()
            .map(|c| {
                c.tbs_certificate
                    .validity
                    .not_after
                    .to_system_time()
                    .duration_since(now + now_offset)
                    .unwrap_or(Duration::ZERO)
            })
            .min()
    }

    pub fn last_request(&self) -> DateTime<Utc> {
        *self.last_request.read().unwrap()
    }

    pub async fn download_certs(&self) -> anyhow::Result<()> {
        let certs = ddnet_account_client::certs::download_certs(self.client.as_ref()).await?;
        let new_account_server_public_keys = certs_to_pub_keys(&certs);
        *self.cur_certs.write().unwrap() = certs;
        *self.last_request.write().unwrap() = chrono::Utc::now();

        *self.account_server_public_keys.write().unwrap() =
            Arc::new(new_account_server_public_keys);

        Ok(())
    }

    pub fn sleep_time(&self) -> Duration {
        let invalid_in = self.invalid_in(SystemTime::now(), Duration::from_secs(7 * 24 * 60 * 60));

        // either if first cert is about to invalidate or when one week passed
        let one_week = Duration::from_secs(7 * 24 * 60 * 60);
        let duration_offset = invalid_in.unwrap_or(one_week).min(one_week);

        let last_request = self.last_request();
        // Check last request is at least one week ago
        let last_request_sys_time = <DateTime<chrono::Local>>::from(last_request);

        let time_diff = chrono::Local::now()
            .signed_duration_since(last_request_sys_time)
            .to_std()
            .unwrap_or(Duration::ZERO);
        if time_diff > one_week {
            duration_offset
        } else {
            // If one week didn't pass, wait at least the remaining time
            duration_offset.max(one_week.saturating_sub(time_diff))
        }
    }

    pub async fn download_task(&self) -> ! {
        loop {
            let duration_offset = self.sleep_time();
            tokio::time::sleep_until(Instant::now() + duration_offset).await;

            match self.download_certs().await {
                Ok(_) => {
                    let certs = self.cur_certs.read().unwrap().clone();
                    let last_request = self.last_request();
                    // write the server certs to file
                    let _ = Self::write_file(&self.client.fs, &certs, last_request).await;
                }
                Err(_) => {
                    // if the download task failed we still want to assure some sleep
                    // of at least one day.
                    let one_week_minus_one_day = TimeDelta::seconds(6 * 24 * 60 * 60);
                    *self.last_request.write().unwrap() = chrono::Utc::now()
                        .checked_sub_signed(one_week_minus_one_day)
                        // but in worst case wait one week
                        .unwrap_or_else(chrono::Utc::now);
                }
            }
        }
    }

    pub fn public_keys(&self) -> Arc<Vec<VerifyingKey>> {
        self.account_server_public_keys.read().unwrap().clone()
    }
}
