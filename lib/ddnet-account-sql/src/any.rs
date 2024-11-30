use futures::future::BoxFuture;
use sqlx::Acquire;

#[derive(Debug)]
/// Enum variant over a database statement
pub enum AnyStatement<'a> {
    #[cfg(feature = "mysql")]
    /// Mysql statement
    MySql(sqlx::mysql::MySqlStatement<'a>),
    /// Sqlite statement
    #[cfg(feature = "sqlite")]
    Sqlite(sqlx::sqlite::SqliteStatement<'a>),
}

/// Enum variant over a database row result
pub enum AnyRow {
    #[cfg(feature = "mysql")]
    /// Mysql query row result
    MySql(sqlx::mysql::MySqlRow),
    /// Sqlite query row result
    #[cfg(feature = "sqlite")]
    Sqlite(sqlx::sqlite::SqliteRow),
}

#[derive(Debug)]
/// Enum variant over a database query result
pub enum AnyQueryResult {
    #[cfg(feature = "mysql")]
    /// Mysql query result
    MySql(sqlx::mysql::MySqlQueryResult),
    /// Sqlite query result
    #[cfg(feature = "sqlite")]
    Sqlite(sqlx::sqlite::SqliteQueryResult),
}

impl AnyQueryResult {
    /// How many rows were affected by the last query.
    pub fn rows_affected(&self) -> u64 {
        match self {
            #[cfg(feature = "mysql")]
            Self::MySql(res) => res.rows_affected(),
            #[cfg(feature = "sqlite")]
            Self::Sqlite(res) => res.rows_affected(),
        }
    }
}

/// Enum variant over a database query
pub enum AnyQuery<'a> {
    #[cfg(feature = "mysql")]
    /// Mysql query
    MySql(sqlx::query::Query<'a, sqlx::MySql, sqlx::mysql::MySqlArguments>),
    /// Sqlite query
    #[cfg(feature = "sqlite")]
    Sqlite(sqlx::query::Query<'a, sqlx::Sqlite, sqlx::sqlite::SqliteArguments<'a>>),
}

impl AnyQuery<'_> {
    /// Executes the given query.
    ///
    /// See [sqlx::query::Query::execute].
    #[allow(irrefutable_let_patterns)]
    pub async fn execute(self, con: &mut AnyConnection<'_>) -> Result<AnyQueryResult, sqlx::Error> {
        Ok(match self {
            #[cfg(feature = "mysql")]
            Self::MySql(query) => {
                let AnyConnection::MySql(con) = con else {
                    return Err(sqlx::Error::AnyDriverError(
                        "Connection was not of mysql type, while query was.".into(),
                    ));
                };
                AnyQueryResult::MySql(query.execute(&mut **con).await?)
            }
            #[cfg(feature = "sqlite")]
            Self::Sqlite(query) => {
                let AnyConnection::Sqlite(con) = con else {
                    return Err(sqlx::Error::AnyDriverError(
                        "Connection was not of sqlite type, while query was.".into(),
                    ));
                };
                AnyQueryResult::Sqlite(query.execute(&mut **con).await?)
            }
        })
    }

    /// Execute the query, returning the first row or [`sqlx::Error::RowNotFound`] otherwise.
    ///
    /// See [sqlx::query::Query::fetch_one].
    #[allow(irrefutable_let_patterns)]
    pub async fn fetch_one(self, con: &mut AnyConnection<'_>) -> Result<AnyRow, sqlx::Error> {
        Ok(match self {
            #[cfg(feature = "mysql")]
            Self::MySql(query) => {
                let AnyConnection::MySql(con) = con else {
                    return Err(sqlx::Error::AnyDriverError(
                        "Connection was not of mysql type, while query was.".into(),
                    ));
                };
                AnyRow::MySql(query.fetch_one(&mut **con).await?)
            }
            #[cfg(feature = "sqlite")]
            Self::Sqlite(query) => {
                let AnyConnection::Sqlite(con) = con else {
                    return Err(sqlx::Error::AnyDriverError(
                        "Connection was not of sqlite type, while query was.".into(),
                    ));
                };
                AnyRow::Sqlite(query.fetch_one(&mut **con).await?)
            }
        })
    }

    /// Execute the query and return all the resulting rows collected into a [`Vec`].
    ///
    /// See [sqlx::query::Query::fetch_all].
    #[allow(irrefutable_let_patterns)]
    pub async fn fetch_all(self, con: &mut AnyConnection<'_>) -> Result<Vec<AnyRow>, sqlx::Error> {
        Ok(match self {
            #[cfg(feature = "mysql")]
            Self::MySql(query) => {
                let AnyConnection::MySql(con) = con else {
                    return Err(sqlx::Error::AnyDriverError(
                        "Connection was not of mysql type, while query was.".into(),
                    ));
                };
                query
                    .fetch_all(&mut **con)
                    .await?
                    .into_iter()
                    .map(AnyRow::MySql)
                    .collect()
            }
            #[cfg(feature = "sqlite")]
            Self::Sqlite(query) => {
                let AnyConnection::Sqlite(con) = con else {
                    return Err(sqlx::Error::AnyDriverError(
                        "Connection was not of sqlite type, while query was.".into(),
                    ));
                };
                query
                    .fetch_all(&mut **con)
                    .await?
                    .into_iter()
                    .map(AnyRow::Sqlite)
                    .collect()
            }
        })
    }

    /// Execute the query, returning the first row or `None` otherwise.
    ///
    /// See [sqlx::query::Query::fetch_optional].
    #[allow(irrefutable_let_patterns)]
    pub async fn fetch_optional(
        self,
        con: &mut AnyConnection<'_>,
    ) -> Result<Option<AnyRow>, sqlx::Error> {
        Ok(match self {
            #[cfg(feature = "mysql")]
            Self::MySql(query) => {
                let AnyConnection::MySql(con) = con else {
                    return Err(sqlx::Error::AnyDriverError(
                        "Connection was not of mysql type, while query was.".into(),
                    ));
                };
                query.fetch_optional(&mut **con).await?.map(AnyRow::MySql)
            }
            #[cfg(feature = "sqlite")]
            Self::Sqlite(query) => {
                let AnyConnection::Sqlite(con) = con else {
                    return Err(sqlx::Error::AnyDriverError(
                        "Connection was not of sqlite type, while query was.".into(),
                    ));
                };
                query.fetch_optional(&mut **con).await?.map(AnyRow::Sqlite)
            }
        })
    }
}

#[derive(Debug)]
/// Enum variant over a database transaction
pub enum AnyTransaction<'a, 'b> {
    #[cfg(feature = "mysql")]
    /// Mysql connection transaction
    MySql(&'a mut sqlx::Transaction<'b, sqlx::MySql>),
    /// Sqlite connection transaction
    #[cfg(feature = "sqlite")]
    Sqlite(&'a mut sqlx::Transaction<'b, sqlx::Sqlite>),
}

impl AnyTransaction<'_, '_> {
    /// Get the connection from this transaction
    pub fn con(&mut self) -> AnyConnection<'_> {
        match self {
            #[cfg(feature = "mysql")]
            AnyTransaction::MySql(trans) => AnyConnection::MySql(trans),
            #[cfg(feature = "sqlite")]
            AnyTransaction::Sqlite(trans) => AnyConnection::Sqlite(trans),
        }
    }
}

#[derive(Debug)]
/// Enum variant over a database connection
pub enum AnyConnection<'a> {
    #[cfg(feature = "mysql")]
    /// Mysql connection
    MySql(&'a mut sqlx::mysql::MySqlConnection),
    /// Sqlite connection
    #[cfg(feature = "sqlite")]
    Sqlite(&'a mut sqlx::sqlite::SqliteConnection),
}

impl AnyConnection<'_> {
    /// Execute the function inside a transaction.
    /// See [`sqlx::Connection::transaction`].
    pub async fn transaction<'a, F, R, E>(&'a mut self, callback: F) -> Result<R, E>
    where
        for<'c> F: FnOnce(AnyTransaction<'c, '_>) -> BoxFuture<'c, Result<R, E>> + 'a + Send + Sync,
        Self: Sized,
        R: Send,
        E: From<sqlx::Error> + Send,
    {
        use sqlx::Connection;
        match self {
            #[cfg(feature = "mysql")]
            AnyConnection::MySql(con) => {
                con.transaction(|transaction| callback(AnyTransaction::MySql(transaction)))
                    .await
            }
            #[cfg(feature = "sqlite")]
            AnyConnection::Sqlite(con) => {
                con.transaction(|transaction| callback(AnyTransaction::Sqlite(transaction)))
                    .await
            }
        }
    }
}

#[derive(Debug)]
/// Enum variant over a database pooled connection
pub enum AnyPoolConnection {
    #[cfg(feature = "mysql")]
    /// Mysql pool connection
    MySql(sqlx::pool::PoolConnection<sqlx::MySql>),
    /// Sqlite pool connection
    #[cfg(feature = "sqlite")]
    Sqlite(sqlx::pool::PoolConnection<sqlx::Sqlite>),
}

impl AnyPoolConnection {
    /// Retrieves the inner connection of this pool connection.
    ///
    /// See [sqlx::Acquire::acquire].
    pub async fn acquire(&mut self) -> Result<AnyConnection, sqlx::Error> {
        Ok(match self {
            #[cfg(feature = "mysql")]
            Self::MySql(con) => AnyConnection::MySql(con.acquire().await?),
            #[cfg(feature = "sqlite")]
            Self::Sqlite(con) => AnyConnection::Sqlite(con.acquire().await?),
        })
    }
}

#[derive(Debug, Clone)]
/// Enum variant over a database connection pool
pub enum AnyPool {
    #[cfg(feature = "mysql")]
    /// Mysql pool
    MySql(sqlx::mysql::MySqlPool),
    /// Sqlite pool
    #[cfg(feature = "sqlite")]
    Sqlite(sqlx::sqlite::SqlitePool),
}

impl AnyPool {
    /// Retrieves a connection from the pool.
    ///
    /// See [sqlx::pool::Pool::acquire].
    pub async fn acquire(&self) -> Result<AnyPoolConnection, sqlx::Error> {
        Ok(match self {
            #[cfg(feature = "mysql")]
            Self::MySql(pool) => AnyPoolConnection::MySql(pool.acquire().await?),
            #[cfg(feature = "sqlite")]
            Self::Sqlite(pool) => AnyPoolConnection::Sqlite(pool.acquire().await?),
        })
    }
}
