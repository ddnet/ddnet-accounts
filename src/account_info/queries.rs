use anyhow::anyhow;
use axum::async_trait;
use ddnet_account_sql::query::Query;
use ddnet_accounts_shared::client::machine_id::MachineUid;
use ddnet_accounts_types::account_id::AccountId;
use sqlx::Executor;
use sqlx::Row;
use sqlx::Statement;

pub struct AccountInfo<'a> {
    pub session_pub_key: &'a [u8; 32],
    pub session_hw_id: &'a MachineUid,
}

pub struct AccountInfoData {
    pub account_id: AccountId,
    pub creation_date: sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>,
    pub linked_email: Option<String>,
    pub linked_steam: Option<i64>,
}

#[async_trait]
impl Query<AccountInfoData> for AccountInfo<'_> {
    async fn prepare_mysql(
        connection: &mut sqlx::mysql::MySqlConnection,
    ) -> anyhow::Result<sqlx::mysql::MySqlStatement<'static>> {
        Ok(connection
            .prepare(include_str!("mysql/account_info.sql"))
            .await?)
    }
    fn query_mysql<'b>(
        &'b self,
        statement: &'b sqlx::mysql::MySqlStatement<'static>,
    ) -> sqlx::query::Query<'b, sqlx::MySql, sqlx::mysql::MySqlArguments> {
        statement
            .query()
            .bind(self.session_pub_key.as_slice())
            .bind(self.session_hw_id.as_slice())
    }
    fn row_data_mysql(row: &sqlx::mysql::MySqlRow) -> anyhow::Result<AccountInfoData> {
        Ok(AccountInfoData {
            account_id: row
                .try_get("account_id")
                .map_err(|err| anyhow!("Failed get column account_id: {err}"))?,
            creation_date: row
                .try_get("creation_date")
                .map_err(|err| anyhow!("Failed get column creation_date: {err}"))?,
            linked_email: row
                .try_get("linked_email")
                .map_err(|err| anyhow!("Failed get column linked_email: {err}"))?,
            linked_steam: row
                .try_get("linked_steam")
                .map_err(|err| anyhow!("Failed get column linked_steam: {err}"))?,
        })
    }
}
