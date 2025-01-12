use std::{sync::Arc, time::Duration};

use ddnet_account_client_http_fs::cert_downloader::CertsDownloader;
use ddnet_account_client_reqwest::client::ClientReqwestTokioFs;
use parking_lot::Mutex;

use crate::{
    certs::{generate_key_and_cert_impl, get_certs, store_cert, PrivateKeys},
    generate_new_signing_keys_impl,
    tests::types::TestAccServer,
};

/// Tests for creating signing certs, checking their validity.
/// Downloading their public keys.
#[tokio::test]
async fn signing_certs() {
    let test = async move {
        // account server setup
        let token: Arc<Mutex<String>> = Default::default();
        let reset_code: Arc<Mutex<String>> = Default::default();
        let acc_server = TestAccServer::new(token.clone(), reset_code.clone(), false, true).await?;

        let (key, cert) = generate_key_and_cert_impl(Duration::from_secs(5))?;
        let now = cert.tbs_certificate.validity.not_before.to_system_time();

        store_cert(&acc_server.shared.db, &acc_server.pool, &cert).await?;
        let certs = get_certs(&acc_server.shared.db, &acc_server.pool).await?;
        // make sure our new cert is stored
        assert!(certs.contains(&cert));

        // also test the downloader task
        let secure_dir_client = tempfile::tempdir()?;
        let client = ClientReqwestTokioFs::new(
            vec!["http://localhost:4433".try_into()?],
            secure_dir_client.path(),
        )
        .await?;
        let downloaded_certs =
            ddnet_account_client::certs::download_certs(client.client.as_ref()).await?;
        assert!(!downloaded_certs.contains(&cert));

        let cert_downloader = CertsDownloader::new(client.client.clone()).await?;
        let invalid_in = cert_downloader.invalid_in(now, Duration::from_secs(0));
        // default certs are valid for at least 1 day
        assert!(invalid_in.is_some_and(|i| i > Duration::from_secs(60 * 60 * 24)));

        *acc_server.shared.cert_chain.write() = Arc::new(certs);
        let downloaded_certs =
            ddnet_account_client::certs::download_certs(client.client.as_ref()).await?;
        assert!(downloaded_certs.contains(&cert));

        // the sleep time is around one week, since no request was made before (the file didn't exist).
        assert!(cert_downloader.sleep_time() > Duration::from_secs(60 * 60 * 24));
        let last_request = cert_downloader.last_request();

        // now force download the newest certs
        cert_downloader.download_certs().await?;

        // the sleep time is around one week, since a request was just made.
        assert!(cert_downloader.sleep_time() > Duration::from_secs(60 * 60 * 24));
        // also make sure the request is reset properly.
        assert!(cert_downloader.last_request() >= last_request);

        // this what the sleep time in the cert downloader would do
        let invalid_in = cert_downloader.invalid_in(now, Duration::from_secs(7 * 24 * 60 * 60));
        let one_week = Duration::from_secs(7 * 24 * 60 * 60);
        let duration_offset = invalid_in.unwrap_or(one_week).min(one_week);
        // since our cert is only valid for 5 seconds, it must have been invalid in one week.
        assert!(duration_offset == Duration::ZERO);
        // but the sleep time is still bigger due to the last request
        assert!(cert_downloader.sleep_time() > Duration::from_secs(60 * 60 * 24));

        // now the cert should be invalid in the previously specified time (now)
        let invalid_in = cert_downloader.invalid_in(now, Duration::from_secs(0));
        // cert must be invalid in exactly 5 seconds from `now`
        assert!(invalid_in.is_some_and(|i| i == Duration::from_secs(5)));

        // if an offset of 5 seconds is used, the cert should now be invalid
        let invalid_in = cert_downloader.invalid_in(now, Duration::from_secs(5));
        assert!(invalid_in.is_some_and(|i| i == Duration::from_secs(0)));

        // if the cert was already invalid, then it should still return 0
        let invalid_in = cert_downloader.invalid_in(now, Duration::from_secs(15));
        assert!(invalid_in.is_some_and(|i| i == Duration::from_secs(0)));

        *acc_server.shared.signing_keys.write() = Arc::new(PrivateKeys {
            current_key: key.clone(),
            current_cert: cert.clone(),
            next_key: key,
            next_cert: cert,
        });

        let default_check_key_time = Duration::from_secs(5);
        let err_check_key_time = Duration::from_secs(5);
        let validy_extra_offset = Duration::from_secs(1);

        // Make sure the recently generated cert is valid
        let res = generate_new_signing_keys_impl(
            &acc_server.pool,
            &acc_server.shared,
            now,
            default_check_key_time,
            err_check_key_time,
            validy_extra_offset,
        )
        .await;
        assert!(matches!(res, either::Either::Left(_)));

        // Make sure the offset is correct
        let res = generate_new_signing_keys_impl(
            &acc_server.pool,
            &acc_server.shared,
            now + default_check_key_time - validy_extra_offset - Duration::from_nanos(1),
            default_check_key_time,
            err_check_key_time,
            validy_extra_offset,
        )
        .await;
        assert!(matches!(res, either::Either::Left(_)));

        // Make sure the offset would trigger generating a new key
        let res = generate_new_signing_keys_impl(
            &acc_server.pool,
            &acc_server.shared,
            now + default_check_key_time - validy_extra_offset,
            default_check_key_time,
            err_check_key_time,
            validy_extra_offset,
        )
        .await;
        assert!(matches!(res, either::Either::Right(_)));

        acc_server.destroy().await?;

        anyhow::Ok(())
    };
    test.await.unwrap();
}
