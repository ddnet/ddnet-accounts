use ddnet_account_sql::query::Query;
use ddnet_accounts_shared::client::credential_auth_token::CredentialAuthTokenOperation;
use ddnet_accounts_shared::client::login::CredentialAuthToken;
use anyhow::anyhow;
use sqlx::any::AnyRow;
use sqlx::Executor;
use sqlx::Statement;

use crate::types::TokenType;

#[derive(Debug)]
pub struct AddCredentialAuthToken<'a> {
    pub token: &'a CredentialAuthToken,
    pub ty: &'a TokenType,
    pub identifier: &'a str,
    pub op: &'a CredentialAuthTokenOperation,
}

#[async_trait::async_trait]
impl<'a> Query<()> for AddCredentialAuthToken<'a> {
    async fn prepare_mysql(
        connection: &mut sqlx::AnyConnection,
    ) -> anyhow::Result<sqlx::any::AnyStatement<'static>> {
        Ok(connection
            .prepare(include_str!("mysql/add_credential_auth_token.sql"))
            .await?)
    }
    fn query_mysql<'b>(
        &'b self,
        statement: &'b sqlx::any::AnyStatement<'static>,
    ) -> sqlx::query::Query<'b, sqlx::Any, sqlx::any::AnyArguments<'b>> {
        let ty: &'static str = self.ty.into();
        let op: &'static str = self.op.into();
        statement
            .query()
            .bind(self.token.as_slice())
            .bind(ty)
            .bind(self.identifier)
            .bind(op)
    }
    fn row_data(_row: &AnyRow) -> anyhow::Result<()> {
        Err(anyhow!("Row data is not supported"))
    }
}
