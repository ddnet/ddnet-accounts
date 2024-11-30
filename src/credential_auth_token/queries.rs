use anyhow::anyhow;
use ddnet_account_sql::query::Query;
use ddnet_accounts_shared::client::credential_auth_token::CredentialAuthTokenOperation;
use ddnet_accounts_shared::client::login::CredentialAuthToken;
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
impl Query<()> for AddCredentialAuthToken<'_> {
    async fn prepare_mysql(
        connection: &mut sqlx::mysql::MySqlConnection,
    ) -> anyhow::Result<sqlx::mysql::MySqlStatement<'static>> {
        Ok(connection
            .prepare(include_str!("mysql/add_credential_auth_token.sql"))
            .await?)
    }
    fn query_mysql<'b>(
        &'b self,
        statement: &'b sqlx::mysql::MySqlStatement<'static>,
    ) -> sqlx::query::Query<'b, sqlx::MySql, sqlx::mysql::MySqlArguments> {
        let ty: &'static str = self.ty.into();
        let op: &'static str = self.op.into();
        statement
            .query()
            .bind(self.token.as_slice())
            .bind(ty)
            .bind(self.identifier)
            .bind(op)
    }
    fn row_data_mysql(_row: &sqlx::mysql::MySqlRow) -> anyhow::Result<()> {
        Err(anyhow!("Row data is not supported"))
    }
}
