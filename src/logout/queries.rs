use ddnet_account_sql::query::Query;
use ddnet_accounts_shared::client::machine_id::MachineUid;
use anyhow::anyhow;
use axum::async_trait;
use sqlx::any::AnyRow;
use sqlx::Executor;
use sqlx::Statement;

pub struct RemoveSession<'a> {
    pub pub_key: &'a [u8; 32],
    pub hw_id: &'a MachineUid,
}

#[async_trait]
impl<'a> Query<()> for RemoveSession<'a> {
    async fn prepare_mysql(
        connection: &mut sqlx::AnyConnection,
    ) -> anyhow::Result<sqlx::any::AnyStatement<'static>> {
        Ok(connection
            .prepare(include_str!("mysql/rem_session.sql"))
            .await?)
    }
    fn query_mysql<'b>(
        &'b self,
        statement: &'b sqlx::any::AnyStatement<'static>,
    ) -> sqlx::query::Query<'b, sqlx::Any, sqlx::any::AnyArguments<'b>> {
        statement
            .query()
            .bind(self.pub_key.as_slice())
            .bind(self.hw_id.as_slice())
    }
    fn row_data(_row: &AnyRow) -> anyhow::Result<()> {
        Err(anyhow!("Row data is not supported"))
    }
}
