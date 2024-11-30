use anyhow::anyhow;
use axum::async_trait;
use ddnet_account_sql::query::Query;
use sqlx::any::AnyRow;
use sqlx::Executor;
use sqlx::Statement;

pub struct UnlinkCredentialByEmail<'a> {
    pub email: &'a email_address::EmailAddress,
}

#[async_trait]
impl Query<()> for UnlinkCredentialByEmail<'_> {
    async fn prepare_mysql(
        connection: &mut sqlx::AnyConnection,
    ) -> anyhow::Result<sqlx::any::AnyStatement<'static>> {
        Ok(connection
            .prepare(include_str!("mysql/unlink_credential_email.sql"))
            .await?)
    }
    fn query_mysql<'b>(
        &'b self,
        statement: &'b sqlx::any::AnyStatement<'static>,
    ) -> sqlx::query::Query<'b, sqlx::Any, sqlx::any::AnyArguments<'b>> {
        statement.query().bind(self.email.as_str())
    }
    fn row_data(_row: &AnyRow) -> anyhow::Result<()> {
        Err(anyhow!("Row data is not supported"))
    }
}

pub struct UnlinkCredentialBySteam<'a> {
    pub steamid64: &'a i64,
}

#[async_trait]
impl Query<()> for UnlinkCredentialBySteam<'_> {
    async fn prepare_mysql(
        connection: &mut sqlx::AnyConnection,
    ) -> anyhow::Result<sqlx::any::AnyStatement<'static>> {
        Ok(connection
            .prepare(include_str!("mysql/unlink_credential_steam.sql"))
            .await?)
    }
    fn query_mysql<'b>(
        &'b self,
        statement: &'b sqlx::any::AnyStatement<'static>,
    ) -> sqlx::query::Query<'b, sqlx::Any, sqlx::any::AnyArguments<'b>> {
        statement.query().bind(self.steamid64)
    }
    fn row_data(_row: &AnyRow) -> anyhow::Result<()> {
        Err(anyhow!("Row data is not supported"))
    }
}
