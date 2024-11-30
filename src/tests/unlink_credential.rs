use std::{str::FromStr, sync::Arc};

use ddnet_account_client::{
    certs::{certs_to_pub_keys, download_certs},
    unlink_credential::UnlinkCredentialResult,
};
use ddnet_account_client_reqwest::client::ClientReqwestTokioFs;
use ddnet_accounts_shared::{
    client::{
        account_token::AccountTokenOperation, credential_auth_token::CredentialAuthTokenOperation,
    },
    game_server,
};
use email_address::EmailAddress;
use parking_lot::Mutex;

use crate::tests::types::TestAccServer;

/// Tests related to verifying that unlinking credential does
/// what it should and fails appropriately
#[tokio::test]
async fn unlink_credential_hardening() {
    let test = async move {
        let secure_dir_client = tempfile::tempdir()?;
        // account server setup
        let token: Arc<Mutex<String>> = Default::default();
        let account_token: Arc<Mutex<String>> = Default::default();
        let acc_server =
            TestAccServer::new(token.clone(), account_token.clone(), false, true).await?;

        let client = ClientReqwestTokioFs::new(
            vec!["http://localhost:4433".try_into()?],
            secure_dir_client.path(),
        )
        .await?;

        let certs = download_certs(&*client).await?;
        let keys = certs_to_pub_keys(&certs);

        // create an account
        ddnet_account_client::credential_auth_token::credential_auth_token_email(
            EmailAddress::from_str("test@localhost")?,
            CredentialAuthTokenOperation::Login,
            None,
            &*client,
        )
        .await?;
        let token_hex = token.lock().clone();
        ddnet_account_client::login::login(token_hex.clone(), &*client)
            .await?
            .1
            .write(&*client)
            .await?;

        let cert = ddnet_account_client::sign::sign(&*client).await?;
        let user_id = game_server::user_id::user_id_from_cert(&keys, cert.certificate_der);

        // try to unlink the account even tho it has only 1 linked credential
        ddnet_account_client::credential_auth_token::credential_auth_token_email(
            EmailAddress::from_str("test@localhost")?,
            CredentialAuthTokenOperation::UnlinkCredential,
            None,
            &*client,
        )
        .await?;
        let credential_auth_token_hex = token.lock().clone();
        let res = ddnet_account_client::unlink_credential::unlink_credential(
            credential_auth_token_hex,
            &*client,
        )
        .await;
        assert!(matches!(
            res,
            Err(UnlinkCredentialResult::AccountServerRequstError(_))
        ));

        // link steam to the account
        ddnet_account_client::account_token::account_token_email(
            EmailAddress::from_str("test@localhost")?,
            AccountTokenOperation::LinkCredential,
            None,
            &*client,
        )
        .await?;
        let account_token_hex = account_token.lock().clone();
        let credential_auth_token_hex =
            ddnet_account_client::credential_auth_token::credential_auth_token_steam(
                b"justatest".to_vec(),
                CredentialAuthTokenOperation::LinkCredential,
                None,
                &*client,
            )
            .await?;
        ddnet_account_client::link_credential::link_credential(
            account_token_hex,
            credential_auth_token_hex,
            &*client,
        )
        .await?;

        // try to unlink the email again
        ddnet_account_client::credential_auth_token::credential_auth_token_email(
            EmailAddress::from_str("test@localhost")?,
            CredentialAuthTokenOperation::UnlinkCredential,
            None,
            &*client,
        )
        .await?;
        let credential_auth_token_hex = token.lock().clone();
        let res = ddnet_account_client::unlink_credential::unlink_credential(
            credential_auth_token_hex,
            &*client,
        )
        .await;
        assert!(res.is_ok());

        // this should create a new account
        ddnet_account_client::credential_auth_token::credential_auth_token_email(
            EmailAddress::from_str("test@localhost")?,
            CredentialAuthTokenOperation::Login,
            None,
            &*client,
        )
        .await?;
        let token_hex = token.lock().clone();
        ddnet_account_client::login::login(token_hex.clone(), &*client)
            .await?
            .1
            .write(&*client)
            .await?;

        let cert = ddnet_account_client::sign::sign(&*client).await?;
        let new_user_id = game_server::user_id::user_id_from_cert(&keys, cert.certificate_der);

        assert!(user_id.account_id != new_user_id.account_id);

        // try to unlink steam from the account, which fails, bcs steam is the only credential
        let credential_auth_token_hex =
            ddnet_account_client::credential_auth_token::credential_auth_token_steam(
                b"justatest".to_vec(),
                CredentialAuthTokenOperation::UnlinkCredential,
                None,
                &*client,
            )
            .await?;
        let res = ddnet_account_client::unlink_credential::unlink_credential(
            credential_auth_token_hex,
            &*client,
        )
        .await;
        assert!(matches!(
            res,
            Err(UnlinkCredentialResult::AccountServerRequstError(_))
        ));

        // link an email to the account again
        let account_token_hex = ddnet_account_client::account_token::account_token_steam(
            b"justatest".to_vec(),
            AccountTokenOperation::LinkCredential,
            None,
            &*client,
        )
        .await?;
        ddnet_account_client::credential_auth_token::credential_auth_token_email(
            EmailAddress::from_str("test2@localhost")?,
            CredentialAuthTokenOperation::LinkCredential,
            None,
            &*client,
        )
        .await?;
        let credential_auth_token_hex = token.lock().clone();
        ddnet_account_client::link_credential::link_credential(
            account_token_hex,
            credential_auth_token_hex,
            &*client,
        )
        .await?;

        // Now unlinking steam from the account should work
        let credential_auth_token_hex =
            ddnet_account_client::credential_auth_token::credential_auth_token_steam(
                b"justatest".to_vec(),
                CredentialAuthTokenOperation::UnlinkCredential,
                None,
                &*client,
            )
            .await?;
        let res = ddnet_account_client::unlink_credential::unlink_credential(
            credential_auth_token_hex,
            &*client,
        )
        .await;
        assert!(res.is_ok());

        acc_server.destroy().await?;

        anyhow::Ok(())
    };
    test.await.unwrap();
}
