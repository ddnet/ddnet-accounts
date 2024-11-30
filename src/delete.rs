pub mod queries;

use std::sync::Arc;

use axum::Json;
use ddnet_account_sql::{any::AnyPool, query::Query};
use ddnet_accounts_shared::{
    account_server::{
        errors::{AccountServerRequestError, Empty},
        result::AccountServerReqResult,
    },
    client::delete::DeleteRequest,
};

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
    let mut connection = connection.acquire().await?;

    connection
        .transaction(|mut connection| {
            Box::pin(async move {
                // token data
                let acc_token_qry = AccountTokenQry {
                    token: &data.account_token,
                };

                let row = acc_token_qry
                    .query(&shared.db.account_token_qry_statement)
                    .fetch_one(&mut connection.con())
                    .await?;

                let token_data = AccountTokenQry::row_data(&row)?;

                // invalidate token
                let qry = InvalidateAccountToken {
                    token: &data.account_token,
                };
                qry.query(&shared.db.invalidate_account_token_statement)
                    .execute(&mut connection.con())
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

                qry.query(&shared.db.remove_sessions_except_statement)
                    .execute(&mut connection.con())
                    .await?;

                // Unlink all credentials
                let qry = UnlinkCredentialEmail {
                    account_id: &account_id,
                };
                qry.query(&shared.db.unlink_credential_email_statement)
                    .execute(&mut connection.con())
                    .await?;

                let qry = UnlinkCredentialSteam {
                    account_id: &account_id,
                };
                qry.query(&shared.db.unlink_credential_steam_statement)
                    .execute(&mut connection.con())
                    .await?;

                // delete account
                let qry = RemoveAccount {
                    account_id: &account_id,
                };

                qry.query(&shared.db.remove_account_statement)
                    .execute(&mut connection.con())
                    .await?;

                anyhow::Ok(())
            })
        })
        .await?;

    Ok(())
}
