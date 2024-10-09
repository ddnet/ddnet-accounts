pub mod queries;

use std::{str::FromStr, sync::Arc};

use axum::Json;
use ddnet_account_sql::{is_duplicate_entry, query::Query};
use ddnet_accounts_shared::{
    account_server::{
        errors::{AccountServerRequestError, Empty},
        result::AccountServerReqResult,
    },
    client::{
        credential_auth_token::CredentialAuthTokenOperation, link_credential::LinkCredentialRequest,
    },
};
use queries::{UnlinkCredentialEmail, UnlinkCredentialSteam};
use sqlx::{Acquire, AnyPool, Connection};

use crate::{
    account_token::queries::{AccountTokenQry, InvalidateAccountToken},
    login::{
        get_and_invalidate_credential_auth_token,
        queries::{LinkAccountCredentialEmail, LinkAccountCredentialSteam},
    },
    shared::Shared,
    types::{AccountTokenType, TokenType},
};

pub async fn link_credential_request(
    shared: Arc<Shared>,
    pool: AnyPool,
    Json(data): Json<LinkCredentialRequest>,
) -> Json<AccountServerReqResult<(), Empty>> {
    Json(link_credential(shared, pool, data).await.map_err(|err| {
        AccountServerRequestError::Unexpected {
            target: "link_credential".into(),
            err: err.to_string(),
            bt: err.backtrace().to_string(),
        }
    }))
}

pub async fn link_credential(
    shared: Arc<Shared>,
    pool: AnyPool,
    data: LinkCredentialRequest,
) -> anyhow::Result<()> {
    let mut connection = pool.acquire().await?;
    let connection = connection.acquire().await?;

    connection
        .transaction(|connection| {
            Box::pin(async move {
                // token data
                let acc_token_qry = AccountTokenQry {
                    token: &data.account_token,
                };

                let row = acc_token_qry
                    .query(connection, &shared.db.account_token_qry_statement)
                    .fetch_one(&mut **connection)
                    .await?;

                let token_data = AccountTokenQry::row_data(&row)?;

                // invalidate token
                let qry = InvalidateAccountToken {
                    token: &data.account_token,
                };
                qry.query(connection, &shared.db.invalidate_account_token_statement)
                    .execute(&mut **connection)
                    .await?;

                anyhow::ensure!(
                    token_data.ty == AccountTokenType::LinkCredential,
                    "Account token was not for logout all operation."
                );
                let account_id = token_data.account_id;

                let token_data = get_and_invalidate_credential_auth_token(
                    &shared,
                    data.credential_auth_token,
                    connection,
                )
                .await?
                .ok_or_else(|| anyhow::anyhow!("Credential auth token is invalid/expired."))?;
                anyhow::ensure!(
                    token_data.op == CredentialAuthTokenOperation::LinkCredential,
                    "Credential auth token was not for linking a new credential"
                );

                match token_data.ty {
                    TokenType::Email => {
                        let email = email_address::EmailAddress::from_str(&token_data.identifier)?;
                        // remove the current email, if exists.
                        let qry = UnlinkCredentialEmail {
                            account_id: &account_id,
                        };

                        qry.query(connection, &shared.db.unlink_credential_email_statement)
                            .execute(&mut **connection)
                            .await?;

                        // add the new email.
                        let qry = LinkAccountCredentialEmail {
                            account_id: &account_id,
                            email: &email,
                        };

                        let res = qry
                            .query(connection, &shared.db.link_credentials_email_qry_statement)
                            .execute(&mut **connection)
                            .await;

                        anyhow::ensure!(
                            !is_duplicate_entry(&res),
                            "This email is already used for a different account."
                        );
                        res?;
                    }
                    TokenType::Steam => {
                        let steamid64: i64 = token_data.identifier.parse()?;
                        // remove the current steam, if exists.
                        let qry = UnlinkCredentialSteam {
                            account_id: &account_id,
                        };

                        qry.query(connection, &shared.db.unlink_credential_steam_statement)
                            .execute(&mut **connection)
                            .await?;

                        // add the new steam.
                        let qry = LinkAccountCredentialSteam {
                            account_id: &account_id,
                            steamid64: &steamid64,
                        };

                        let res = qry
                            .query(connection, &shared.db.link_credentials_steam_qry_statement)
                            .execute(&mut **connection)
                            .await;

                        anyhow::ensure!(
                            !is_duplicate_entry(&res),
                            "This email is already used for a different account."
                        );
                        res?;
                    }
                }

                anyhow::Ok(())
            })
        })
        .await?;

    Ok(())
}
