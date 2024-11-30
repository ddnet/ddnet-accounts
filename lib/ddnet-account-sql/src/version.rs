use crate::any::{AnyConnection, AnyPool};

#[cfg(feature = "mysql")]
mod mysql {
    use sqlx::Executor;
    use sqlx::Row;
    use sqlx::Statement;

    pub(super) async fn try_setup(con: &mut sqlx::mysql::MySqlConnection) -> anyhow::Result<()> {
        // first create all statements (syntax check)
        let version = con
            .prepare(include_str!("version/mysql/version.sql"))
            .await?;

        // afterwards actually create tables
        version.query().execute(&mut *con).await?;

        Ok(())
    }

    pub(super) async fn get_or_set_version(
        con: &mut sqlx::mysql::MySqlConnection,
        name: &str,
    ) -> anyhow::Result<i64> {
        // first create all statements (syntax check)
        let get_version = con
            .prepare(include_str!("version/mysql/get_version.sql"))
            .await?;
        let set_version = con
            .prepare(include_str!("version/mysql/set_version.sql"))
            .await?;

        let name = name.to_string();

        // afterwards actually create tables
        if let Some(row) = get_version
            .query()
            .bind(&name)
            .fetch_optional(&mut *con)
            .await?
        {
            anyhow::Ok(row.try_get("version")?)
        } else {
            // insert new entry
            set_version
                .query()
                .bind(&name)
                .bind(0)
                .bind(0)
                .execute(&mut *con)
                .await?;
            anyhow::Ok(0)
        }
    }

    pub(super) async fn set_version(
        con: &mut sqlx::mysql::MySqlConnection,
        name: &str,
        version: i64,
    ) -> anyhow::Result<()> {
        // first create all statements (syntax check)
        let set_version = con
            .prepare(include_str!("version/mysql/set_version.sql"))
            .await?;

        Ok(set_version
            .query()
            .bind(name)
            .bind(version)
            .bind(version)
            .execute(&mut *con)
            .await
            .map(|_| ())?)
    }

    pub(super) async fn delete(con: &mut sqlx::mysql::MySqlConnection) -> anyhow::Result<()> {
        // first create all statements (syntax check)
        // delete in reverse order to creating
        let version = con
            .prepare(include_str!("version/mysql/delete/version.sql"))
            .await?;

        // afterwards actually drop tables
        let version = version.query().execute(&mut *con).await;

        // handle errors at once
        version?;

        Ok(())
    }
}

#[cfg(feature = "sqlite")]
mod sqlite {
    use sqlx::Executor;
    use sqlx::Row;
    use sqlx::Statement;

    pub(super) async fn try_setup(con: &mut sqlx::sqlite::SqliteConnection) -> anyhow::Result<()> {
        // first create all statements (syntax check)
        let version = con
            .prepare(include_str!("version/sqlite/version.sql"))
            .await?;

        // afterwards actually create tables
        version.query().execute(&mut *con).await?;

        Ok(())
    }

    pub(super) async fn get_or_set_version(
        con: &mut sqlx::sqlite::SqliteConnection,
        name: &str,
    ) -> anyhow::Result<i64> {
        // first create all statements (syntax check)
        let get_version = con
            .prepare(include_str!("version/sqlite/get_version.sql"))
            .await?;
        let set_version = con
            .prepare(include_str!("version/sqlite/set_version.sql"))
            .await?;

        let name = name.to_string();

        // afterwards actually create tables
        if let Some(row) = get_version
            .query()
            .bind(&name)
            .fetch_optional(&mut *con)
            .await?
        {
            anyhow::Ok(row.try_get("version")?)
        } else {
            // insert new entry
            set_version
                .query()
                .bind(&name)
                .bind(0)
                .bind(0)
                .execute(&mut *con)
                .await?;
            anyhow::Ok(0)
        }
    }

    pub(super) async fn set_version(
        con: &mut sqlx::sqlite::SqliteConnection,
        name: &str,
        version: i64,
    ) -> anyhow::Result<()> {
        // first create all statements (syntax check)
        let set_version = con
            .prepare(include_str!("version/sqlite/set_version.sql"))
            .await?;

        Ok(set_version
            .query()
            .bind(name)
            .bind(version)
            .bind(version)
            .execute(&mut *con)
            .await
            .map(|_| ())?)
    }

    pub(super) async fn delete(con: &mut sqlx::sqlite::SqliteConnection) -> anyhow::Result<()> {
        // first create all statements (syntax check)
        // delete in reverse order to creating
        let version = con
            .prepare(include_str!("version/sqlite/delete/version.sql"))
            .await?;

        // afterwards actually drop tables
        let version = version.query().execute(&mut *con).await;

        // handle errors at once
        version?;

        Ok(())
    }
}

/// Use this function to obtain the current version number.
///
/// If the version table does not exist, sets up the version table.
/// The version table can be used to easily upgrade existing tables to a new
/// version, without manually doing it by hand.
pub async fn get_version(con: &mut AnyConnection<'_>, name: &str) -> anyhow::Result<i64> {
    match con {
        #[cfg(feature = "mysql")]
        AnyConnection::MySql(con) => {
            // try setup
            mysql::try_setup(con).await?;
            mysql::get_or_set_version(con, name).await
        }
        #[cfg(feature = "sqlite")]
        AnyConnection::Sqlite(con) => {
            // try setup
            sqlite::try_setup(con).await?;
            sqlite::get_or_set_version(con, name).await
        }
    }
}

/// After your setup is done, set the version to your current setup script.
/// This can (and should) be called inside a transaction
pub async fn set_version(
    con: &mut AnyConnection<'_>,
    name: &str,
    version: i64,
) -> anyhow::Result<()> {
    match con {
        #[cfg(feature = "mysql")]
        AnyConnection::MySql(con) => mysql::set_version(con, name, version).await,
        #[cfg(feature = "sqlite")]
        AnyConnection::Sqlite(con) => sqlite::set_version(con, name, version).await,
    }
}

/// Drop the version table...
/// Warning: This is usually not recommended.
pub async fn delete(pool: &AnyPool) -> anyhow::Result<()> {
    let mut con = pool.acquire().await?;
    let con = con.acquire().await?;
    match con {
        #[cfg(feature = "mysql")]
        AnyConnection::MySql(con) => mysql::delete(con).await,
        #[cfg(feature = "sqlite")]
        AnyConnection::Sqlite(con) => sqlite::delete(con).await,
    }
}
