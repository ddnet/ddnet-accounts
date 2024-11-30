use async_trait::async_trait;

use crate::any::{AnyConnection, AnyQuery, AnyRow, AnyStatement};

/// An interface for queries to allow converting them to various database implementations
#[async_trait]
pub trait Query<A> {
    /// MySQL version of [`Query::prepare`].
    #[cfg(feature = "mysql")]
    async fn prepare_mysql(
        connection: &mut sqlx::mysql::MySqlConnection,
    ) -> anyhow::Result<sqlx::mysql::MySqlStatement<'static>>;

    /// Sqlite version of [`Query::prepare`].
    #[cfg(feature = "sqlite")]
    async fn prepare_sqlite(
        connection: &mut sqlx::sqlite::SqliteConnection,
    ) -> anyhow::Result<sqlx::sqlite::SqliteStatement<'static>>;

    /// Prepare a statement to be later used by [`Query::query`].
    async fn prepare(connection: &mut AnyConnection) -> anyhow::Result<AnyStatement<'static>> {
        Ok(match connection {
            #[cfg(feature = "mysql")]
            AnyConnection::MySql(connection) => {
                AnyStatement::MySql(Self::prepare_mysql(connection).await?)
            }
            #[cfg(feature = "sqlite")]
            AnyConnection::Sqlite(connection) => {
                AnyStatement::Sqlite(Self::prepare_sqlite(connection).await?)
            }
        })
    }

    /// MySQL version of [`Query::query`].
    #[cfg(feature = "mysql")]
    fn query_mysql<'a>(
        &'a self,
        statement: &'a sqlx::mysql::MySqlStatement<'static>,
    ) -> sqlx::query::Query<'a, sqlx::MySql, sqlx::mysql::MySqlArguments>;

    /// Sqlite version of [`Query::query`].
    #[cfg(feature = "sqlite")]
    fn query_sqlite<'a>(
        &'a self,
        statement: &'a sqlx::sqlite::SqliteStatement<'static>,
    ) -> sqlx::query::Query<'a, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'a>>;

    /// Get a query with all arguments bound already, ready to be fetched.
    fn query<'a>(&'a self, statement: &'a AnyStatement<'static>) -> AnyQuery<'a> {
        match statement {
            #[cfg(feature = "mysql")]
            AnyStatement::MySql(statement) => AnyQuery::MySql(self.query_mysql(statement)),
            #[cfg(feature = "sqlite")]
            AnyStatement::Sqlite(statement) => AnyQuery::Sqlite(self.query_sqlite(statement)),
        }
    }

    /// MySQL version of [`Query::row_data`].
    #[cfg(feature = "mysql")]
    fn row_data_mysql(row: &sqlx::mysql::MySqlRow) -> anyhow::Result<A>;

    /// Sqlite version of [`Query::row_data`].
    #[cfg(feature = "sqlite")]
    fn row_data_sqlite(row: &sqlx::sqlite::SqliteRow) -> anyhow::Result<A>;

    /// Gets the row data for a result row of this query
    fn row_data(row: &AnyRow) -> anyhow::Result<A> {
        match row {
            #[cfg(feature = "mysql")]
            AnyRow::MySql(row) => Self::row_data_mysql(row),
            #[cfg(feature = "sqlite")]
            AnyRow::Sqlite(row) => Self::row_data_sqlite(row),
        }
    }
}
