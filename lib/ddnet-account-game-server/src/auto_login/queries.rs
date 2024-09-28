use ddnet_account_sql::query::Query;
use ddnet_accounts_types::account_id::AccountId;
use anyhow::anyhow;
use async_trait::async_trait;
use sqlx::any::AnyRow;
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
impl<'a> Query<()> for RegisterUser<'a> {
    async fn prepare_mysql(
        connection: &mut sqlx::AnyConnection,
    ) -> anyhow::Result<sqlx::any::AnyStatement<'static>> {
        Ok(connection
            .prepare(include_str!("mysql/try_insert_user.sql"))
            .await?)
    }
    fn query_mysql<'b>(
        &'b self,
        statement: &'b sqlx::any::AnyStatement<'static>,
    ) -> sqlx::query::Query<'b, sqlx::Any, sqlx::any::AnyArguments<'b>> {
        let account_id = self.account_id;

        statement.query().bind(self.default_name).bind(account_id)
    }
    fn row_data(_row: &AnyRow) -> anyhow::Result<()> {
        Err(anyhow!(
            "Data rows are not supported for this query.
            You probably want to check affected rows instead."
        ))
    }
}
