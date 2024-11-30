use anyhow::anyhow;
use axum::async_trait;
use ddnet_account_sql::query::Query;
use ddnet_accounts_shared::client::machine_id::MachineUid;
use sqlx::Executor;
use sqlx::Statement;

pub struct RemoveSession<'a> {
    pub pub_key: &'a [u8; 32],
    pub hw_id: &'a MachineUid,
}

#[async_trait]
impl Query<()> for RemoveSession<'_> {
    async fn prepare_mysql(
        connection: &mut sqlx::mysql::MySqlConnection,
    ) -> anyhow::Result<sqlx::mysql::MySqlStatement<'static>> {
        Ok(connection
            .prepare(include_str!("mysql/rem_session.sql"))
            .await?)
    }
    fn query_mysql<'b>(
        &'b self,
        statement: &'b sqlx::mysql::MySqlStatement<'static>,
    ) -> sqlx::query::Query<'b, sqlx::MySql, sqlx::mysql::MySqlArguments> {
        statement
            .query()
            .bind(self.pub_key.as_slice())
            .bind(self.hw_id.as_slice())
    }
    fn row_data_mysql(_row: &sqlx::mysql::MySqlRow) -> anyhow::Result<()> {
        Err(anyhow!("Row data is not supported"))
    }
}
