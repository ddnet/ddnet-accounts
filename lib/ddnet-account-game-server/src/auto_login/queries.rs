use anyhow::anyhow;
use async_trait::async_trait;
use ddnet_account_sql::query::Query;
use ddnet_accounts_types::account_id::AccountId;
use sqlx::Executor;
use sqlx::Statement;

/// A query that tries to insert a new user in the database.
/// On failure it does nothing.
#[derive(Debug)]
pub struct RegisterUser<'a> {
    /// the account id of the user, see [`AccountId`]
    pub account_id: &'a AccountId,
    /// the default name of the user
    pub default_name: &'a str,
}

#[async_trait]
impl Query<()> for RegisterUser<'_> {
    #[cfg(feature = "mysql")]
    async fn prepare_mysql(
        connection: &mut sqlx::mysql::MySqlConnection,
    ) -> anyhow::Result<sqlx::mysql::MySqlStatement<'static>> {
        Ok(connection
            .prepare(include_str!("mysql/try_insert_user.sql"))
            .await?)
    }
    #[cfg(feature = "sqlite")]
    async fn prepare_sqlite(
        connection: &mut sqlx::sqlite::SqliteConnection,
    ) -> anyhow::Result<sqlx::sqlite::SqliteStatement<'static>> {
        Ok(connection
            .prepare(include_str!("sqlite/try_insert_user.sql"))
            .await?)
    }
    #[cfg(feature = "mysql")]
    fn query_mysql<'b>(
        &'b self,
        statement: &'b sqlx::mysql::MySqlStatement<'static>,
    ) -> sqlx::query::Query<'b, sqlx::MySql, sqlx::mysql::MySqlArguments> {
        let account_id = self.account_id;

        statement.query().bind(self.default_name).bind(account_id)
    }
    #[cfg(feature = "sqlite")]
    fn query_sqlite<'b>(
        &'b self,
        statement: &'b sqlx::sqlite::SqliteStatement<'static>,
    ) -> sqlx::query::Query<'b, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'b>> {
        let account_id = self.account_id;

        statement.query().bind(self.default_name).bind(account_id)
    }
    #[cfg(feature = "mysql")]
    fn row_data_mysql(_row: &sqlx::mysql::MySqlRow) -> anyhow::Result<()> {
        Err(anyhow!(
            "Data rows are not supported for this query.
            You probably want to check affected rows instead."
        ))
    }
    #[cfg(feature = "sqlite")]
    fn row_data_sqlite(_row: &sqlx::sqlite::SqliteRow) -> anyhow::Result<()> {
        Err(anyhow!(
            "Data rows are not supported for this query.
            You probably want to check affected rows instead."
        ))
    }
}
