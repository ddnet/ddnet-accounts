use std::{
    sync::{Arc, RwLock},
    time::{Duration, SystemTime},
};

use anyhow::anyhow;
use ddnet_account_client::certs::certs_to_pub_keys;
use ddnet_accounts_shared::game_server::user_id::VerifyingKey;
use tokio::time::Instant;
use x509_cert::der::{Decode, Encode};

use crate::client::ClientHttpTokioFs;

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
                serde_json::from_slice::<Vec<Vec<u8>>>(&cert_json)
                    .map_err(|err| anyhow!(err))
                    .and_then(|certs_der| {
                        certs_der
                            .into_iter()
                            .map(|cert_der| {
                                x509_cert::Certificate::from_der(&cert_der)
                                    .map_err(|err| anyhow!(err))
                            })
                            .collect::<anyhow::Result<Vec<x509_cert::Certificate>>>()
                    })
            });

        match certs_file {
            Ok(certs_file) => Ok(Arc::new(Self {
                client,
                account_server_public_keys: RwLock::new(Arc::new(certs_to_pub_keys(&certs_file))),
                cur_certs: RwLock::new(certs_file),
            })),
            Err(_) => {
                // try to download latest cert instead
                let certs = ddnet_account_client::certs::download_certs(client.as_ref()).await?;

                let _ = client
                    .fs
                    .write(
                        "".as_ref(),
                        "account_server_certs.json".as_ref(),
                        serde_json::to_vec(
                            &certs
                                .iter()
                                .map(|cert| cert.to_der().map_err(|err| anyhow!(err)))
                                .collect::<anyhow::Result<Vec<_>>>()?,
                        )?,
                    )
                    .await;

                Ok(Arc::new(Self {
                    account_server_public_keys: RwLock::new(Arc::new(certs_to_pub_keys(&certs))),
                    client,
                    cur_certs: RwLock::new(certs),
                }))
            }
        }
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

    pub async fn download_certs(&self) {
        if let Ok(certs) = ddnet_account_client::certs::download_certs(self.client.as_ref()).await {
            let new_account_server_public_keys = certs_to_pub_keys(&certs);
            *self.cur_certs.write().unwrap() = certs;

            *self.account_server_public_keys.write().unwrap() =
                Arc::new(new_account_server_public_keys);
        }
    }

    pub async fn download_task(&self) -> ! {
        loop {
            let invalid_in =
                self.invalid_in(SystemTime::now(), Duration::from_secs(7 * 24 * 60 * 60));

            // either if first cert is about to invalidate or when one week passed
            let one_week = Duration::from_secs(7 * 24 * 60 * 60);
            let duration_offset = invalid_in.unwrap_or(one_week).min(one_week);

            tokio::time::sleep_until(Instant::now() + duration_offset).await;

            self.download_certs().await;
        }
    }

    pub fn public_keys(&self) -> Arc<Vec<VerifyingKey>> {
        self.account_server_public_keys.read().unwrap().clone()
    }
}
