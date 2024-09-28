use std::{str::FromStr, sync::Arc};

use ddnet_account_client::credential_auth_token::CredentialAuthTokenResult;
use ddnet_account_client_reqwest::client::ClientReqwestTokioFs;
use ddnet_accounts_shared::{
    account_server::errors::AccountServerRequestError,
    client::credential_auth_token::CredentialAuthTokenOperation,
};
use email_address::EmailAddress;
use parking_lot::Mutex;
use url::Host;

use crate::{
    email_limit::{EmailDomainAllowList, EmailDomainDenyList},
    tests::types::TestAccServer,
};

/// Tests for ban list
#[tokio::test]
async fn credential_auth_token_ban_lists() {
    let test = async move {
        let secure_dir_client = tempfile::tempdir()?;
        // account server setup
        let token: Arc<Mutex<String>> = Default::default();
        let reset_code: Arc<Mutex<String>> = Default::default();
        let acc_server = TestAccServer::new(token.clone(), reset_code.clone(), false, true).await?;

        let email = EmailAddress::from_str("test@localhost")?;
        *acc_server.shared.email.deny_list.write() = EmailDomainDenyList {
            domains: vec![Host::parse("localhost")?].into_iter().collect(),
        };

        let client = ClientReqwestTokioFs::new(
            vec!["http://localhost:4433".try_into()?],
            secure_dir_client.path(),
        )
        .await?;

        // localhost is banned
        let res = ddnet_account_client::credential_auth_token::credential_auth_token_email(
            email.clone(),
            CredentialAuthTokenOperation::Login,
            None,
            &*client,
        )
        .await;

        assert!(matches!(
            res.unwrap_err(),
            CredentialAuthTokenResult::AccountServerRequstError(AccountServerRequestError::Other(
                _
            ))
        ));

        *acc_server.shared.email.allow_list.write() = EmailDomainAllowList {
            domains: vec![Host::parse("localhost")?].into_iter().collect(),
        };

        // localhost is allowed, but also still banned. Banning has higher precedence
        let res = ddnet_account_client::credential_auth_token::credential_auth_token_email(
            email.clone(),
            CredentialAuthTokenOperation::Login,
            None,
            &*client,
        )
        .await;

        assert!(matches!(
            res.unwrap_err(),
            CredentialAuthTokenResult::AccountServerRequstError(AccountServerRequestError::Other(
                _
            ))
        ));

        *acc_server.shared.email.deny_list.write() = EmailDomainDenyList::default();
        // localhost is allowed
        let res = ddnet_account_client::credential_auth_token::credential_auth_token_email(
            email.clone(),
            CredentialAuthTokenOperation::Login,
            None,
            &*client,
        )
        .await;
        assert!(res.is_ok());

        *acc_server.shared.email.allow_list.write() = EmailDomainAllowList {
            domains: vec![Host::parse("ddnet.org")?].into_iter().collect(),
        };

        // only ddnet.org is allowed, localhost fails now, since an allow list is in use.
        let res = ddnet_account_client::credential_auth_token::credential_auth_token_email(
            email,
            CredentialAuthTokenOperation::Login,
            None,
            &*client,
        )
        .await;

        assert!(matches!(
            res.unwrap_err(),
            CredentialAuthTokenResult::AccountServerRequstError(AccountServerRequestError::Other(
                _
            ))
        ));

        acc_server.destroy().await?;

        anyhow::Ok(())
    };
    test.await.unwrap();
}
