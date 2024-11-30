use anyhow::anyhow;
use axum::async_trait;
use ddnet_account_sql::query::Query;
use sqlx::Executor;
use sqlx::Statement;

pub struct CleanupCredentialAuthTokens {}

#[async_trait]
impl Query<()> for CleanupCredentialAuthTokens {
    async fn prepare_mysql(
        connection: &mut sqlx::mysql::MySqlConnection,
    ) -> anyhow::Result<sqlx::mysql::MySqlStatement<'static>> {
        Ok(connection
            .prepare(include_str!("mysql/cleanup_credential_auth_tokens.sql"))
            .await?)
    }
    fn query_mysql<'b>(
        &'b self,
        statement: &'b sqlx::mysql::MySqlStatement<'static>,
    ) -> sqlx::query::Query<'b, sqlx::MySql, sqlx::mysql::MySqlArguments> {
        statement.query()
    }
    fn row_data_mysql(_row: &sqlx::mysql::MySqlRow) -> anyhow::Result<()> {
        Err(anyhow!("Row data is not supported"))
    }
}

pub struct CleanupAccountTokens {}

#[async_trait]
impl Query<()> for CleanupAccountTokens {
    async fn prepare_mysql(
        connection: &mut sqlx::mysql::MySqlConnection,
    ) -> anyhow::Result<sqlx::mysql::MySqlStatement<'static>> {
        Ok(connection
            .prepare(include_str!("mysql/cleanup_account_tokens.sql"))
            .await?)
    }
    fn query_mysql<'b>(
        &'b self,
        statement: &'b sqlx::mysql::MySqlStatement<'static>,
    ) -> sqlx::query::Query<'b, sqlx::MySql, sqlx::mysql::MySqlArguments> {
        statement.query()
    }
    fn row_data_mysql(_row: &sqlx::mysql::MySqlRow) -> anyhow::Result<()> {
        Err(anyhow!("Row data is not supported"))
    }
}

pub struct CleanupCerts {}

#[async_trait]
impl Query<()> for CleanupCerts {
    async fn prepare_mysql(
        connection: &mut sqlx::mysql::MySqlConnection,
    ) -> anyhow::Result<sqlx::mysql::MySqlStatement<'static>> {
        Ok(connection
            .prepare(include_str!("mysql/cleanup_certs.sql"))
            .await?)
    }
    fn query_mysql<'b>(
        &'b self,
        statement: &'b sqlx::mysql::MySqlStatement<'static>,
    ) -> sqlx::query::Query<'b, sqlx::MySql, sqlx::mysql::MySqlArguments> {
        statement.query()
    }
    fn row_data_mysql(_row: &sqlx::mysql::MySqlRow) -> anyhow::Result<()> {
        Err(anyhow!("Row data is not supported"))
    }
}
