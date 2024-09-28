use ddnet_account_sql::query::Query;
use ddnet_accounts_shared::client::sign::SignRequest;
use ddnet_accounts_types::account_id::AccountId;
use sqlx::any::AnyRow;
use sqlx::types::chrono::DateTime;
use sqlx::types::chrono::Utc;
use sqlx::Executor;
use sqlx::Row;
use sqlx::Statement;

#[derive(Debug)]
pub struct AuthAttempt<'a> {
    pub data: &'a SignRequest,
}

#[derive(Debug)]
pub struct AuthAttemptData {
    pub account_id: AccountId,
    pub creation_date: DateTime<Utc>,
}

#[async_trait::async_trait]
impl<'a> Query<AuthAttemptData> for AuthAttempt<'a> {
    async fn prepare_mysql(
        connection: &mut sqlx::AnyConnection,
    ) -> anyhow::Result<sqlx::any::AnyStatement<'static>> {
        Ok(connection.prepare(include_str!("mysql/auth.sql")).await?)
    }
    fn query_mysql<'b>(
        &'b self,
        statement: &'b sqlx::any::AnyStatement<'static>,
    ) -> sqlx::query::Query<'b, sqlx::Any, sqlx::any::AnyArguments<'b>> {
        statement
            .query()
            .bind(self.data.account_data.public_key.as_bytes().as_slice())
            .bind(self.data.account_data.hw_id.as_slice())
    }
    fn row_data(row: &AnyRow) -> anyhow::Result<AuthAttemptData> {
        Ok(AuthAttemptData {
            account_id: row.try_get("account_id")?,
            creation_date: row.try_get("create_time")?,
        })
    }
}
