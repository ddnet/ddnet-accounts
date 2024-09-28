use std::sync::Arc;

use ddnet_account_client_reqwest::client::ClientReqwestTokioFs;
use ddnet_accounts_shared::client::credential_auth_token::CredentialAuthTokenOperation;
use parking_lot::Mutex;

use crate::tests::types::TestAccServer;

#[tokio::test]
async fn multi_url_test() {
    let test = async move {
        let secure_dir_client = tempfile::tempdir()?;

        // account server setup
        let token: Arc<Mutex<String>> = Default::default();
        let account_token: Arc<Mutex<String>> = Default::default();
        let acc_server =
            TestAccServer::new(token.clone(), account_token.clone(), false, true).await?;

        // This request should fail.
        let broken_url = "http://localhost:55443";
        let url = "http://localhost:4433";
        let client = ClientReqwestTokioFs::new(
            vec![broken_url.try_into()?, url.try_into()?],
            secure_dir_client.path(),
        )
        .await?;

        assert!(
            client
                .client
                .cur_http
                .load(std::sync::atomic::Ordering::Relaxed)
                == 0
        );
        let _ = ddnet_account_client::certs::download_certs(&*client).await?;
        assert!(
            client
                .client
                .cur_http
                .load(std::sync::atomic::Ordering::Relaxed)
                == 1
        );

        client
            .client
            .cur_http
            .store(0, std::sync::atomic::Ordering::Relaxed);
        let _ = ddnet_account_client::credential_auth_token::credential_auth_token_steam(
            b"justatest".to_vec(),
            CredentialAuthTokenOperation::LinkCredential,
            None,
            &*client,
        )
        .await?;
        assert!(
            client
                .client
                .cur_http
                .load(std::sync::atomic::Ordering::Relaxed)
                == 1
        );

        acc_server.destroy().await?;

        anyhow::Ok(())
    };

    test.await.unwrap()
}
