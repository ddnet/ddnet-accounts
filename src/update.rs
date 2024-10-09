use std::{sync::Arc, time::Duration};

use ddnet_account_sql::query::Query;
use queries::{CleanupAccountTokens, CleanupCerts, CleanupCredentialAuthTokens};
use sqlx::{Acquire, AnyPool, Executor};

use crate::{email::EmailShared, email_limit, ip_limit, shared::Shared};

pub mod queries;

pub async fn update_impl(pool: &AnyPool, shared: &Arc<Shared>) {
    if let Ok(mut connection) = pool.acquire().await {
        if let Ok(connection) = connection.acquire().await {
            // cleanup credential auth tokens
            let _ = connection
                .execute(CleanupCredentialAuthTokens {}.query(
                    connection,
                    &shared.db.cleanup_credential_auth_tokens_statement,
                ))
                .await;

            // cleanup account tokens
            let _ = connection
                .execute(
                    CleanupAccountTokens {}
                        .query(connection, &shared.db.cleanup_account_tokens_statement),
                )
                .await;

            // cleanup certs
            let _ = connection
                .execute(CleanupCerts {}.query(connection, &shared.db.cleanup_certs_statement))
                .await;
        }
    }
}

pub async fn update(pool: AnyPool, shared: Arc<Shared>) -> ! {
    loop {
        update_impl(&pool, &shared).await;

        // only do the update once per hour
        tokio::time::sleep(Duration::from_secs(60 * 60 * 24)).await;
    }
}

pub async fn handle_watchers(shared: Arc<Shared>) {
    let shared_email_deny = shared.clone();
    let shared_email_allow = shared.clone();
    let shared_email_account_tokens = shared.clone();
    let shared_email_credential_auth_tokens = shared.clone();
    let res = tokio::try_join!(
        tokio::spawn(async move {
            let mut ip_ban = ip_limit::IpDenyList::watcher();
            loop {
                if ip_ban.wait_for_change().await.is_ok() {
                    let ip_ban_list = ip_limit::IpDenyList::load_from_file().await;
                    *shared.ip_ban_list.write() = ip_ban_list;
                } else {
                    break;
                }
            }
        }),
        tokio::spawn(async move {
            let mut email_deny = email_limit::EmailDomainDenyList::watcher();
            loop {
                if email_deny.wait_for_change().await.is_ok() {
                    let deny_list = email_limit::EmailDomainDenyList::load_from_file().await;
                    *shared_email_deny.email.deny_list.write() = deny_list;
                } else {
                    break;
                }
            }
        }),
        tokio::spawn(async move {
            let mut email_allow = email_limit::EmailDomainAllowList::watcher();
            loop {
                if email_allow.wait_for_change().await.is_ok() {
                    let allow_list = email_limit::EmailDomainAllowList::load_from_file().await;
                    *shared_email_allow.email.allow_list.write() = allow_list;
                } else {
                    break;
                }
            }
        }),
        tokio::spawn(async move {
            let mut email_account_tokens = EmailShared::watcher("account_tokens.html");
            loop {
                if email_account_tokens.wait_for_change().await.is_ok() {
                    if let Ok(mail) = EmailShared::load_email_template("account_tokens.html").await
                    {
                        *shared_email_account_tokens.account_tokens_email.write() = Arc::new(mail);
                    }
                } else {
                    break;
                }
            }
        }),
        tokio::spawn(async move {
            let mut email_credential_auth = EmailShared::watcher("credential_auth_tokens.html");
            loop {
                if email_credential_auth.wait_for_change().await.is_ok() {
                    if let Ok(mail) =
                        EmailShared::load_email_template("credential_auth_tokens.html").await
                    {
                        *shared_email_credential_auth_tokens
                            .credential_auth_tokens_email
                            .write() = Arc::new(mail);
                    }
                } else {
                    break;
                }
            }
        }),
    );
    if let Err(err) = res {
        log::error!("{err}");
    }
}
