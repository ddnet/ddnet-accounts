use std::{
    net::{Ipv4Addr, Ipv6Addr},
    str::FromStr,
    sync::Arc,
};

use ddnet_account_client::credential_auth_token::CredentialAuthTokenResult;
use ddnet_accounts_shared::{
    account_server::errors::AccountServerRequestError,
    client::credential_auth_token::CredentialAuthTokenOperation,
};
use ddnet_account_client_reqwest::client::ClientReqwestTokioFs;
use email_address::EmailAddress;
use iprange::IpRange;
use parking_lot::Mutex;

use crate::{ip_limit::IpDenyList, tests::types::TestAccServer};

/// Tests for ip ban list
#[tokio::test]
async fn ip_ban_lists() {
    let test = async move {
        let secure_dir_client = tempfile::tempdir()?;
        // account server setup
        let token: Arc<Mutex<String>> = Default::default();
        let reset_code: Arc<Mutex<String>> = Default::default();
        let acc_server = TestAccServer::new(token.clone(), reset_code.clone(), false, true).await?;

        let email = EmailAddress::from_str("test@localhost")?;
        *acc_server.shared.ip_ban_list.write() = IpDenyList {
            ipv4: {
                let mut ip = IpRange::new();

                ip.add(Ipv4Addr::from_str("127.0.0.1")?.into());

                ip
            },
            ipv6: {
                let mut ip = IpRange::new();

                ip.add(Ipv6Addr::from_str("::1")?.into());

                ip
            },
        };

        let client = ClientReqwestTokioFs::new(
            vec!["http://localhost:4433".try_into()?],
            secure_dir_client.path(),
        )
        .await?;

        // 127.0.0.1 is banned
        let res = ddnet_account_client::credential_auth_token::credential_auth_token_email(
            email,
            CredentialAuthTokenOperation::Login,
            None,
            &*client,
        )
        .await;

        assert!(matches!(
            res.unwrap_err(),
            CredentialAuthTokenResult::AccountServerRequstError(AccountServerRequestError::VpnBan(
                _
            ))
        ));

        acc_server.destroy().await?;

        anyhow::Ok(())
    };
    test.await.unwrap();
}
