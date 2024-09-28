use std::{
    str::FromStr,
    sync::Arc,
    time::{Duration, SystemTime},
};

use anyhow::anyhow;
use ddnet_account_client::{
    certs::{certs_to_pub_keys, download_certs},
    logout::logout,
    sign::SignResult,
};
use ddnet_account_client_reqwest::client::ClientReqwestTokioFs;
use ddnet_accounts_shared::{
    account_server::{account_info::CredentialType, cert_account_ext::AccountCertExt},
    client::{
        account_token::AccountTokenOperation, credential_auth_token::CredentialAuthTokenOperation,
    },
    game_server,
};
use email_address::EmailAddress;
use parking_lot::Mutex;
use x509_cert::der::Decode;

use crate::{
    certs::PrivateKeys,
    generate_new_signing_keys,
    tests::types::{TestAccServer, TestGameServer},
    update::update_impl,
};

#[tokio::test]
async fn account_full_process() {
    let test = async move {
        let secure_dir_client = tempfile::tempdir()?;

        // account server setup
        let token: Arc<Mutex<String>> = Default::default();
        let account_token: Arc<Mutex<String>> = Default::default();
        let acc_server =
            TestAccServer::new(token.clone(), account_token.clone(), false, true).await?;
        let pool = acc_server.pool.clone();
        let shared = acc_server.shared.clone();

        let url = "http://localhost:4433";
        let client =
            ClientReqwestTokioFs::new(vec![url.try_into()?], secure_dir_client.path()).await?;

        let login = || {
            Box::pin(async {
                ddnet_account_client::credential_auth_token::credential_auth_token_email(
                    EmailAddress::from_str("test@localhost")?,
                    CredentialAuthTokenOperation::Login,
                    None,
                    &*client,
                )
                .await?;

                // do actual login for client
                let token_hex = token.lock().clone();
                let account_data = ddnet_account_client::login::login(token_hex, &*client).await?;
                anyhow::Ok(account_data)
            })
        };
        // the first login will also create the account
        login().await?.1.write(&*client).await?;

        // create a current signed certificate on the account server
        let cert = ddnet_account_client::sign::sign(&*client).await?;

        let Ok(Some((_, account_data))) = x509_cert::Certificate::from_der(&cert.certificate_der)?
            .tbs_certificate
            .get::<AccountCertExt>()
        else {
            return Err(anyhow!("no valid account data found."));
        };

        assert!(account_data.data.account_id >= 1);

        // now comes game server
        let game_server = TestGameServer::new(&pool).await?;
        let game_server_data = game_server.game_server_data.clone();

        // emulate a game server that downloads certs from account server to validate
        // the account cert from the client.
        let certs = download_certs(&*client).await?;
        let keys = certs_to_pub_keys(&certs);

        // Now use the client cert to get the user id, which is either the account id
        // or the public key fingerprint
        let user_id = game_server::user_id::user_id_from_cert(&keys, cert.certificate_der);
        assert!(user_id.account_id.is_some());

        // What the game server usually does is to provide a mechanism for the client
        // to auto login the user, this automatically registers new users (if it has a valid account id).
        // And in case of an "upgrade" so that a user previously had no account id but
        // uses the same public key again, it will move the points of this public key
        // to that account.
        let auto_login_res = ddnet_account_game_server::auto_login::auto_login(
            game_server_data.clone(),
            &pool,
            &user_id,
        )
        .await;
        assert!(auto_login_res.is_ok_and(|v| v));
        // Logging in again simply will not create a new account, but otherwise works
        let auto_login_res = ddnet_account_game_server::auto_login::auto_login(
            game_server_data.clone(),
            &pool,
            &user_id,
        )
        .await;
        assert!(auto_login_res.is_ok_and(|v| !v));
        ddnet_account_game_server::rename::rename(
            game_server_data.clone(),
            &pool,
            &user_id,
            "nameless_tee",
        )
        .await?;

        // remove this session
        logout(&*client).await?;

        // signing should fail now
        assert!(matches!(
            ddnet_account_client::sign::sign(&*client).await,
            Err(SignResult::FsLikeError(_))
        ));

        // login again
        login().await?.1.write(&*client).await?;

        let account_info = ddnet_account_client::account_info::account_info(&*client).await?;
        assert!(account_info
            .credentials
            .iter()
            .any(|c| if let CredentialType::Email(mail) = c {
                mail == "t**t@l*******t"
            } else {
                false
            }));

        // since the next step is only a logout, the user id must stay
        // the same
        let cert = ddnet_account_client::sign::sign(&*client).await?;
        let before_logout_user_id =
            game_server::user_id::user_id_from_cert(&keys, cert.certificate_der);

        // remove all sessions except the current one
        ddnet_account_client::account_token::account_token_email(
            EmailAddress::from_str("test@localhost")?,
            AccountTokenOperation::LogoutAll,
            None,
            &*client,
        )
        .await?;
        let account_token_hex = account_token.lock().clone();
        ddnet_account_client::logout_all::logout_all(account_token_hex, &*client).await?;

        // signing should still work
        assert!(ddnet_account_client::sign::sign(&*client).await.is_ok(),);

        // login again
        login().await?.1.write(&*client).await?;

        let cert = ddnet_account_client::sign::sign(&*client).await?;
        let after_logout_user_id =
            game_server::user_id::user_id_from_cert(&keys, cert.certificate_der);
        // make sure the account itself is still valid
        assert!(after_logout_user_id.account_id == before_logout_user_id.account_id);

        // delete account
        ddnet_account_client::account_token::account_token_email(
            EmailAddress::from_str("test@localhost")?,
            AccountTokenOperation::Delete,
            None,
            &*client,
        )
        .await?;
        let account_token_hex = account_token.lock().clone();
        ddnet_account_client::delete::delete(account_token_hex, &*client).await?;

        // signing should fail now
        assert!(matches!(
            ddnet_account_client::sign::sign(&*client).await,
            Err(SignResult::FsLikeError(_))
        ));

        // login again, should create a new account
        login().await?.1.write(&*client).await?;
        let cert = ddnet_account_client::sign::sign(&*client).await?;
        let after_delete_user_id =
            game_server::user_id::user_id_from_cert(&keys, cert.certificate_der);
        // make sure the account itself was deleted properly, account ids should differ
        assert!(after_logout_user_id.account_id != after_delete_user_id.account_id);

        // use link credential as rename for the email
        ddnet_account_client::account_token::account_token_email(
            EmailAddress::from_str("test@localhost")?,
            AccountTokenOperation::LinkCredential,
            None,
            &*client,
        )
        .await?;
        let account_token_hex = account_token.lock().clone();
        ddnet_account_client::credential_auth_token::credential_auth_token_email(
            EmailAddress::from_str("test2@localhost")?,
            CredentialAuthTokenOperation::LinkCredential,
            None,
            &*client,
        )
        .await?;
        let token_hex = token.lock().clone();
        ddnet_account_client::link_credential::link_credential(
            account_token_hex,
            token_hex,
            &*client,
        )
        .await?;

        // login with test2 should work
        ddnet_account_client::credential_auth_token::credential_auth_token_email(
            EmailAddress::from_str("test2@localhost")?,
            CredentialAuthTokenOperation::Login,
            None,
            &*client,
        )
        .await?;

        // do actual login for client
        let token_hex = token.lock().clone();
        ddnet_account_client::login::login(token_hex, &*client)
            .await?
            .1
            .write(&*client)
            .await?;

        let cert = ddnet_account_client::sign::sign(&*client).await?;
        let after_link_credential_user_id =
            game_server::user_id::user_id_from_cert(&keys, cert.certificate_der);
        assert!(after_link_credential_user_id.account_id == after_delete_user_id.account_id);

        let credential_auth_token_hex =
            ddnet_account_client::credential_auth_token::credential_auth_token_steam(
                b"justatest".to_vec(),
                CredentialAuthTokenOperation::Login,
                None,
                &*client,
            )
            .await?;
        ddnet_account_client::login::login(credential_auth_token_hex, &*client)
            .await?
            .1
            .write(&*client)
            .await?;

        let link_email = || {
            Box::pin(async {
                let account_token_hex = ddnet_account_client::account_token::account_token_steam(
                    b"justatest".to_vec(),
                    AccountTokenOperation::LinkCredential,
                    None,
                    &*client,
                )
                .await?;
                ddnet_account_client::credential_auth_token::credential_auth_token_email(
                    EmailAddress::from_str("test@localhost")?,
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
                anyhow::Ok(())
            })
        };
        link_email().await?;

        // unlink email
        ddnet_account_client::credential_auth_token::credential_auth_token_email(
            EmailAddress::from_str("test@localhost")?,
            CredentialAuthTokenOperation::UnlinkCredential,
            None,
            &*client,
        )
        .await?;
        let credential_auth_token_hex = token.lock().clone();
        ddnet_account_client::unlink_credential::unlink_credential(
            credential_auth_token_hex,
            &*client,
        )
        .await?;

        link_email().await?;

        // unlink steam
        let credential_auth_token_hex =
            ddnet_account_client::credential_auth_token::credential_auth_token_steam(
                b"justatest".to_vec(),
                CredentialAuthTokenOperation::UnlinkCredential,
                None,
                &*client,
            )
            .await?;
        ddnet_account_client::unlink_credential::unlink_credential(
            credential_auth_token_hex,
            &*client,
        )
        .await?;

        game_server.destroy().await?;
        // game server end

        // test some account server related stuff
        // updates (which usually do cleanup tasks)
        update_impl(&pool, &shared).await;

        // generate new signing keys
        let cur_keys = shared.signing_keys.read().clone();
        let mut fake_cert = cur_keys.current_cert.clone();
        fake_cert.tbs_certificate.validity.not_after = SystemTime::now().try_into().unwrap();
        let fake_keys = PrivateKeys {
            current_key: cur_keys.current_key.clone(),
            current_cert: fake_cert,
            next_key: cur_keys.next_key.clone(),
            next_cert: cur_keys.next_cert.clone(),
        };
        *shared.signing_keys.write() = Arc::new(fake_keys);
        generate_new_signing_keys(&pool, &shared).await;

        // if above worked both keys should be around same lifetime
        let cur_keys = shared.signing_keys.read().clone();
        // assumes that this test never runs for a whole day...
        anyhow::ensure!(
            cur_keys
                .current_cert
                .tbs_certificate
                .validity
                .not_after
                .to_system_time()
                + Duration::from_secs(60 * 60 * 24)
                > cur_keys
                    .next_cert
                    .tbs_certificate
                    .validity
                    .not_after
                    .to_system_time(),
            "certs do not have a similar lifetime"
        );

        acc_server.destroy().await?;

        anyhow::Ok(())
    };

    test.await.unwrap()
}
