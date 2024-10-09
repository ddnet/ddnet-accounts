use sqlx::Acquire;
use sqlx::AnyConnection;
use sqlx::Executor;
use sqlx::Row;
use sqlx::Statement;

async fn try_setup_generic(con: &mut AnyConnection, sql: &'static str) -> anyhow::Result<()> {
    // first create all statements (syntax check)
    let version = con.prepare(sql).await?;

    // afterwards actually create tables
    version.query().execute(&mut *con).await?;

    Ok(())
}

async fn get_or_set_version_generic(
    con: &mut AnyConnection,
    name: &str,
    get_version_sql: &'static str,
    set_version_sql: &'static str,
) -> anyhow::Result<i64> {
    // first create all statements (syntax check)
    let get_version = con.prepare(get_version_sql).await?;
    let set_version = con.prepare(set_version_sql).await?;

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

async fn set_version_generic(
    con: &mut AnyConnection,
    name: &str,
    version: i64,
    set_version_sql: &'static str,
) -> anyhow::Result<()> {
    // first create all statements (syntax check)
    let set_version = con.prepare(set_version_sql).await?;

    Ok(set_version
        .query()
        .bind(name)
        .bind(version)
        .bind(version)
        .execute(&mut *con)
        .await
        .map(|_| ())?)
}

async fn delete_genric(pool: &sqlx::AnyPool, delete_sql: &'static str) -> anyhow::Result<()> {
    let mut pool_con = pool.acquire().await?;
    let con = pool_con.acquire().await?;

    // first create all statements (syntax check)
    // delete in reverse order to creating
    let version = con.prepare(delete_sql).await?;

    // afterwards actually drop tables
    let version = version.query().execute(&mut *con).await;

    // handle errors at once
    version?;

    Ok(())
}

#[cfg(feature = "mysql")]
mod mysql {
    use sqlx::AnyConnection;

    pub(super) async fn try_setup(con: &mut AnyConnection) -> anyhow::Result<()> {
        super::try_setup_generic(con, include_str!("version/mysql/version.sql")).await
    }

    pub(super) async fn get_or_set_version(
        con: &mut AnyConnection,
        name: &str,
    ) -> anyhow::Result<i64> {
        super::get_or_set_version_generic(
            con,
            name,
            include_str!("version/mysql/get_version.sql"),
            include_str!("version/mysql/set_version.sql"),
        )
        .await
    }

    pub(super) async fn set_version(
        con: &mut AnyConnection,
        name: &str,
        version: i64,
    ) -> anyhow::Result<()> {
        super::set_version_generic(
            con,
            name,
            version,
            include_str!("version/mysql/set_version.sql"),
        )
        .await
    }

    pub(super) async fn delete(pool: &sqlx::AnyPool) -> anyhow::Result<()> {
        super::delete_genric(pool, include_str!("version/mysql/delete/version.sql")).await
    }
}

#[cfg(feature = "sqlite")]
mod sqlite {
    use sqlx::AnyConnection;
    pub(super) async fn try_setup(con: &mut AnyConnection) -> anyhow::Result<()> {
        super::try_setup_generic(con, include_str!("version/sqlite/version.sql")).await
    }

    pub(super) async fn get_or_set_version(
        con: &mut AnyConnection,
        name: &str,
    ) -> anyhow::Result<i64> {
        super::get_or_set_version_generic(
            con,
            name,
            include_str!("version/sqlite/get_version.sql"),
            include_str!("version/sqlite/set_version.sql"),
        )
        .await
    }

    pub(super) async fn set_versione(
        con: &mut AnyConnection,
        name: &str,
        version: i64,
    ) -> anyhow::Result<()> {
        super::set_version_generic(
            con,
            name,
            version,
            include_str!("version/sqlite/set_version.sql"),
        )
        .await
    }

    pub(super) async fn delete(pool: &sqlx::AnyPool) -> anyhow::Result<()> {
        super::delete_genric(pool, include_str!("version/sqlite/delete/version.sql")).await
    }
}

/// Use this function to obtain the current version number.
///
/// If the version table does not exist, sets up the version table.
/// The version table can be used to easily upgrade existing tables to a new
/// version, without manually doing it by hand.
pub async fn get_version(con: &mut AnyConnection, name: &str) -> anyhow::Result<i64> {
    match con.kind() {
        #[cfg(feature = "mysql")]
        sqlx::any::AnyKind::MySql => {
            // try setup
            mysql::try_setup(con).await?;
            mysql::get_or_set_version(con, name).await
        }
        #[cfg(feature = "sqlite")]
        sqlx::any::AnyKind::Sqlite => {
            // try setup
            sqlite::try_setup(con).await?;
            sqlite::get_or_set_version(con, name).await
        }
    }
}

/// After your setup is done, set the version to your current setup script.
/// This can (and should) be called inside a transaction
pub async fn set_version(con: &mut AnyConnection, name: &str, version: i64) -> anyhow::Result<()> {
    match con.kind() {
        #[cfg(feature = "mysql")]
        sqlx::any::AnyKind::MySql => mysql::set_version(con, name, version).await,
        #[cfg(feature = "sqlite")]
        sqlx::any::AnyKind::Sqlite => sqlite::set_versione(con, name, version).await,
    }
}

/// Drop the version table...
/// Warning: This is usually not recommended.
pub async fn delete(pool: &sqlx::AnyPool) -> anyhow::Result<()> {
    match pool.any_kind() {
        #[cfg(feature = "mysql")]
        sqlx::any::AnyKind::MySql => mysql::delete(pool).await,
        #[cfg(feature = "sqlite")]
        sqlx::any::AnyKind::Sqlite => sqlite::delete(pool).await,
    }
}
