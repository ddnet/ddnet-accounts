use ddnet_account_sql::query::Query;
use ddnet_accounts_types::account_id::AccountId;
use anyhow::anyhow;
use axum::async_trait;
use sqlx::any::AnyRow;
use sqlx::Executor;
use sqlx::Statement;

pub struct RemoveAccount<'a> {
    pub account_id: &'a AccountId,
}

#[async_trait]
impl<'a> Query<()> for RemoveAccount<'a> {
    async fn prepare_mysql(
        connection: &mut sqlx::AnyConnection,
    ) -> anyhow::Result<sqlx::any::AnyStatement<'static>> {
        Ok(connection
            .prepare(include_str!("mysql/rem_account.sql"))
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
