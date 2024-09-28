use ddnet_account_sql::query::Query;
use ddnet_accounts_shared::client::account_data::AccountDataForServer;
use ddnet_accounts_types::account_id::AccountId;
use anyhow::anyhow;
use axum::async_trait;
use sqlx::any::AnyRow;
use sqlx::Executor;
use sqlx::Statement;

pub struct RemoveSessionsExcept<'a> {
    pub account_id: &'a AccountId,
    pub session_data: &'a Option<AccountDataForServer>,
}

#[async_trait]
impl<'a> Query<()> for RemoveSessionsExcept<'a> {
    async fn prepare_mysql(
        connection: &mut sqlx::AnyConnection,
    ) -> anyhow::Result<sqlx::any::AnyStatement<'static>> {
        Ok(connection
            .prepare(include_str!("mysql/rem_sessions_except.sql"))
            .await?)
    }
    fn query_mysql<'b>(
        &'b self,
        statement: &'b sqlx::any::AnyStatement<'static>,
    ) -> sqlx::query::Query<'b, sqlx::Any, sqlx::any::AnyArguments<'b>> {
        let (key, hwid) = self
            .session_data
            .as_ref()
            .map(|data| (data.public_key.as_bytes().as_slice(), data.hw_id.as_slice()))
            .unzip();
        statement
            .query()
            .bind(self.account_id)
            .bind(key)
            .bind(key)
            .bind(hwid)
    }
    fn row_data(_row: &AnyRow) -> anyhow::Result<()> {
        Err(anyhow!("Row data is not supported"))
    }
}
