use sqlx::Acquire;
use sqlx::AnyConnection;
use sqlx::Executor;
use sqlx::Row;
use sqlx::Statement;

async fn try_setup_mysql(con: &mut AnyConnection) -> anyhow::Result<()> {
    // first create all statements (syntax check)
    let version = con
        .prepare(include_str!("version/mysql/version.sql"))
        .await?;

    // afterwards actually create tables
    version.query().execute(&mut *con).await?;

    Ok(())
}

async fn get_or_set_version_mysql(con: &mut AnyConnection, name: &str) -> anyhow::Result<i64> {
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

async fn set_version_mysql(
    con: &mut AnyConnection,
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

/// Use this function to obtain the current version number.
///
/// If the version table does not exist, sets up the version table.
/// The version table can be used to easily upgrade existing tables to a new
/// version, without manually doing it by hand.
pub async fn get_version(con: &mut AnyConnection, name: &str) -> anyhow::Result<i64> {
    match con.kind() {
        sqlx::any::AnyKind::MySql => {
            // try setup
            try_setup_mysql(con).await?;
            get_or_set_version_mysql(con, name).await
        }
    }
}

/// After your setup is done, set the version to your current setup script.
/// This can (and should) be called inside a transaction
pub async fn set_version(con: &mut AnyConnection, name: &str, version: i64) -> anyhow::Result<()> {
    match con.kind() {
        sqlx::any::AnyKind::MySql => set_version_mysql(con, name, version).await,
    }
}

async fn delete_mysql(pool: &sqlx::AnyPool) -> anyhow::Result<()> {
    let mut pool_con = pool.acquire().await?;
    let con = pool_con.acquire().await?;

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

/// Drop the version table...
/// Warning: This is usually not recommended.
pub async fn delete(pool: &sqlx::AnyPool) -> anyhow::Result<()> {
    match pool.any_kind() {
        sqlx::any::AnyKind::MySql => delete_mysql(pool).await,
    }
}
