use std::{str::FromStr, sync::Arc};

use ddnet_account_client::{credential_auth_token::CredentialAuthTokenResult, login::LoginResult};
use ddnet_accounts_shared::{
    account_server::{errors::AccountServerRequestError, login::LoginError},
    client::credential_auth_token::CredentialAuthTokenOperation,
};
use ddnet_account_client_reqwest::client::ClientReqwestTokioFs;
use email_address::EmailAddress;
use parking_lot::Mutex;

use crate::tests::types::TestAccServer;

/// Tests related to [`CredentialAuthTokenResult`] & [`LoginResult`] & server side login
#[tokio::test]
async fn login_rate_limit() {
    let test = async move {
        let secure_dir_client = tempfile::tempdir()?;
        // account server setup
        let token: Arc<Mutex<String>> = Default::default();
        let reset_code: Arc<Mutex<String>> = Default::default();
        let acc_server = TestAccServer::new(token.clone(), reset_code.clone(), true, true).await?;

        let client = ClientReqwestTokioFs::new(
            vec!["http://localhost:4433".try_into()?],
            secure_dir_client.path(),
        )
        .await?;

        ddnet_account_client::credential_auth_token::credential_auth_token_email(
            EmailAddress::from_str("test@localhost")?,
            CredentialAuthTokenOperation::Login,
            None,
            &*client,
        )
        .await?;

        // do actual login for client
        let token_hex = token.lock().clone();
        ddnet_account_client::login::login(token_hex.clone(), &*client)
            .await?
            .1
            .write(&*client)
            .await?;

        let err = ddnet_account_client::credential_auth_token::credential_auth_token_email(
            EmailAddress::from_str("test@localhost")?,
            CredentialAuthTokenOperation::Login,
            None,
            &*client,
        )
        .await
        .unwrap_err();
        assert!(matches!(
            err,
            CredentialAuthTokenResult::AccountServerRequstError(
                AccountServerRequestError::RateLimited(_)
            )
        ));

        let _ = ddnet_account_client::login::login(token_hex.clone(), &*client).await;
        let _ = ddnet_account_client::login::login(token_hex.clone(), &*client).await;
        let _ = ddnet_account_client::login::login(token_hex.clone(), &*client).await;
        let _ = ddnet_account_client::login::login(token_hex.clone(), &*client).await;
        let _ = ddnet_account_client::login::login(token_hex.clone(), &*client).await;
        // After the 5th attempt it should rate limit
        let err = ddnet_account_client::login::login(token_hex.clone(), &*client)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            LoginResult::AccountServerRequstError(AccountServerRequestError::RateLimited(_))
        ));

        acc_server.destroy().await?;

        anyhow::Ok(())
    };
    test.await.unwrap();
}

#[tokio::test]
async fn login_hardening() {
    let test = async move {
        let secure_dir_client = tempfile::tempdir()?;
        // account server setup
        let token: Arc<Mutex<String>> = Default::default();
        let reset_code: Arc<Mutex<String>> = Default::default();
        let acc_server = TestAccServer::new(token.clone(), reset_code.clone(), false, true).await?;

        let client = ClientReqwestTokioFs::new(
            vec!["http://localhost:4433".try_into()?],
            secure_dir_client.path(),
        )
        .await?;

        // don't allow emails with display name or ips
        let res = ddnet_account_client::credential_auth_token::credential_auth_token_email(
            EmailAddress::from_str("Name <test@localhost>")?,
            CredentialAuthTokenOperation::Login,
            None,
            &*client,
        )
        .await;
        assert!(matches!(
            res,
            Err(CredentialAuthTokenResult::AccountServerRequstError(_))
        ));
        let res = ddnet_account_client::credential_auth_token::credential_auth_token_email(
            EmailAddress::from_str("test@[127.0.0.1]")?,
            CredentialAuthTokenOperation::Login,
            None,
            &*client,
        )
        .await;
        assert!(matches!(
            res,
            Err(CredentialAuthTokenResult::AccountServerRequstError(_))
        ));

        ddnet_account_client::credential_auth_token::credential_auth_token_email(
            EmailAddress::from_str("test@localhost")?,
            CredentialAuthTokenOperation::Login,
            None,
            &*client,
        )
        .await?;

        let token_hex = token.lock().clone();
        // already use the token
        ddnet_account_client::login::login(token_hex.clone(), &*client)
            .await?
            .1
            .write(&*client)
            .await?;

        let err = ddnet_account_client::login::login("invalid".to_string(), &*client)
            .await
            .unwrap_err();
        assert!(matches!(err, LoginResult::Other(_)));

        // token can't be valid at this point anymore
        let err = ddnet_account_client::login::login(token_hex, &*client)
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            LoginResult::AccountServerRequstError(AccountServerRequestError::LogicError(
                LoginError::TokenInvalid
            ))
        ));

        acc_server.destroy().await?;

        anyhow::Ok(())
    };
    test.await.unwrap();
}

#[tokio::test]
async fn login_email_test() {
    let test = async move {
        let secure_dir_client = tempfile::tempdir()?;
        // account server setup
        let token: Arc<Mutex<String>> = Default::default();
        let reset_code: Arc<Mutex<String>> = Default::default();
        let acc_server =
            TestAccServer::new(token.clone(), reset_code.clone(), false, false).await?;

        let client = ClientReqwestTokioFs::new(
            vec!["http://localhost:4433".try_into()?],
            secure_dir_client.path(),
        )
        .await?;

        // localhost is forbidden, since email_test_mode is false in TestAccServer::new
        let res = ddnet_account_client::credential_auth_token::credential_auth_token_email(
            EmailAddress::from_str("test@localhost")?,
            CredentialAuthTokenOperation::Login,
            None,
            &*client,
        )
        .await;
        assert!(matches!(
            res,
            Err(CredentialAuthTokenResult::AccountServerRequstError(_))
        ));

        acc_server.destroy().await?;

        anyhow::Ok(())
    };
    test.await.unwrap();
}
