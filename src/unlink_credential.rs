pub mod queries;

use std::{str::FromStr, sync::Arc};

use axum::Json;
use ddnet_account_sql::{any::AnyPool, query::Query};
use ddnet_accounts_shared::{
    account_server::{
        errors::{AccountServerRequestError, Empty},
        result::AccountServerReqResult,
    },
    client::unlink_credential::UnlinkCredentialRequest,
};
use queries::{UnlinkCredentialByEmail, UnlinkCredentialBySteam};

use crate::{
    login::get_and_invalidate_credential_auth_token,
    shared::Shared,
    types::{CredentialAuthTokenType, TokenType},
};

pub async fn unlink_credential_request(
    shared: Arc<Shared>,
    pool: AnyPool,
    Json(data): Json<UnlinkCredentialRequest>,
) -> Json<AccountServerReqResult<(), Empty>> {
    Json(unlink_credential(shared, pool, data).await.map_err(|err| {
        AccountServerRequestError::Unexpected {
            target: "unlink_credential".into(),
            err: err.to_string(),
            bt: err.backtrace().to_string(),
        }
    }))
}

pub async fn unlink_credential(
    shared: Arc<Shared>,
    pool: AnyPool,
    data: UnlinkCredentialRequest,
) -> anyhow::Result<()> {
    let mut connection = pool.acquire().await?;
    let mut connection = connection.acquire().await?;

    connection
        .transaction(|mut connection| {
            Box::pin(async move {
                let token_data = get_and_invalidate_credential_auth_token(
                    &shared,
                    data.credential_auth_token,
                    &mut connection.con(),
                )
                .await?
                .ok_or_else(|| anyhow::anyhow!("Credential auth token is invalid/expired."))?;
                anyhow::ensure!(
                    token_data.op == CredentialAuthTokenType::UnlinkCredential,
                    "Credential auth token was not for unlinking \
                    the current credential from its account"
                );

                let affected_rows = match token_data.ty {
                    TokenType::Email => {
                        let email = email_address::EmailAddress::from_str(&token_data.identifier)?;
                        // remove the current email, if exists.
                        let qry = UnlinkCredentialByEmail { email: &email };

                        qry.query(&shared.db.unlink_credential_by_email_statement)
                            .execute(&mut connection.con())
                            .await?
                            .rows_affected()
                    }
                    TokenType::Steam => {
                        let steamid64: i64 = token_data.identifier.parse()?;
                        // remove the current steam, if exists.
                        let qry = UnlinkCredentialBySteam {
                            steamid64: &steamid64,
                        };

                        qry.query(&shared.db.unlink_credential_by_steam_statement)
                            .execute(&mut connection.con())
                            .await?
                            .rows_affected()
                    }
                };

                anyhow::ensure!(
                    affected_rows > 0,
                    "No credential was unlinked. \
                    There has to be at least one credential per account."
                );

                anyhow::Ok(())
            })
        })
        .await?;

    Ok(())
}
