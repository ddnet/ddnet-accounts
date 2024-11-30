use anyhow::anyhow;
use axum::async_trait;
use ddnet_account_sql::query::Query;
use ddnet_accounts_types::account_id::AccountId;
use sqlx::any::AnyRow;
use sqlx::Executor;
use sqlx::Statement;

pub struct UnlinkCredentialEmail<'a> {
    pub account_id: &'a AccountId,
}

#[async_trait]
impl Query<()> for UnlinkCredentialEmail<'_> {
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
        statement.query().bind(self.account_id)
    }
    fn row_data(_row: &AnyRow) -> anyhow::Result<()> {
        Err(anyhow!("Row data is not supported"))
    }
}

pub struct UnlinkCredentialSteam<'a> {
    pub account_id: &'a AccountId,
}

#[async_trait]
impl Query<()> for UnlinkCredentialSteam<'_> {
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
        statement.query().bind(self.account_id)
    }
    fn row_data(_row: &AnyRow) -> anyhow::Result<()> {
        Err(anyhow!("Row data is not supported"))
    }
}
