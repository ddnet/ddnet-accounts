use async_trait::async_trait;
use sqlx::any::{AnyKind, AnyRow};

/// An interface for queries to allow converting them to various database implementations
#[async_trait]
pub trait Query<A> {
    /// MySQL version of [`Query::prepare`].
    async fn prepare_mysql(
        connection: &mut sqlx::AnyConnection,
    ) -> anyhow::Result<sqlx::any::AnyStatement<'static>>;

    /// Prepare a statement to be later used by [`Query::query`].
    async fn prepare(
        connection: &mut sqlx::AnyConnection,
    ) -> anyhow::Result<sqlx::any::AnyStatement<'static>> {
        match connection.kind() {
            AnyKind::MySql => Self::prepare_mysql(connection).await,
            //_ => Err(anyhow!("database backend not implemented.")),
        }
    }

    /// Get a query with all arguments bound already, ready to be fetched.
    fn query_mysql<'a>(
        &'a self,
        statement: &'a sqlx::any::AnyStatement<'static>,
    ) -> sqlx::query::Query<'a, sqlx::Any, sqlx::any::AnyArguments<'a>>;

    /// Get a query with all arguments bound already, ready to be fetched.
    fn query<'a>(
        &'a self,
        statement: &'a sqlx::any::AnyStatement<'static>,
    ) -> sqlx::query::Query<'a, sqlx::Any, sqlx::any::AnyArguments<'a>> {
        self.query_mysql(statement)
    }

    /// Gets the row data for a result row of this query
    fn row_data(row: &AnyRow) -> anyhow::Result<A>;
}
