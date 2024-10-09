pub mod queries;

use std::sync::Arc;

use axum::Json;
use ddnet_account_sql::query::Query;
use ddnet_accounts_shared::{
    account_server::{
        errors::{AccountServerRequestError, Empty},
        result::AccountServerReqResult,
    },
    client::delete::DeleteRequest,
};
use sqlx::{Acquire, AnyPool, Connection};

use crate::{
    account_token::queries::{AccountTokenQry, InvalidateAccountToken},
    link_credential::queries::{UnlinkCredentialEmail, UnlinkCredentialSteam},
    logout_all::queries::RemoveSessionsExcept,
    shared::Shared,
    types::AccountTokenType,
};

use self::queries::RemoveAccount;

pub async fn delete_request(
    shared: Arc<Shared>,
    pool: AnyPool,
    Json(data): Json<DeleteRequest>,
) -> Json<AccountServerReqResult<(), Empty>> {
    Json(
        delete(shared, pool, data)
            .await
            .map_err(|err| AccountServerRequestError::Unexpected {
                target: "delete_request".into(),
                err: err.to_string(),
                bt: err.backtrace().to_string(),
            }),
    )
}

pub async fn delete(shared: Arc<Shared>, pool: AnyPool, data: DeleteRequest) -> anyhow::Result<()> {
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
                    token_data.ty == AccountTokenType::Delete,
                    "Account token was not for delete operation."
                );
                let account_id = token_data.account_id;

                // remove all sessions
                let qry = RemoveSessionsExcept {
                    account_id: &account_id,
                    session_data: &None,
                };

                qry.query(connection, &shared.db.remove_sessions_except_statement)
                    .execute(&mut **connection)
                    .await?;

                // Unlink all credentials
                let qry = UnlinkCredentialEmail {
                    account_id: &account_id,
                };
                qry.query(connection, &shared.db.unlink_credential_email_statement)
                    .execute(&mut **connection)
                    .await?;

                let qry = UnlinkCredentialSteam {
                    account_id: &account_id,
                };
                qry.query(connection, &shared.db.unlink_credential_steam_statement)
                    .execute(&mut **connection)
                    .await?;

                // delete account
                let qry = RemoveAccount {
                    account_id: &account_id,
                };

                qry.query(connection, &shared.db.remove_account_statement)
                    .execute(&mut **connection)
                    .await?;

                anyhow::Ok(())
            })
        })
        .await?;

    Ok(())
}
