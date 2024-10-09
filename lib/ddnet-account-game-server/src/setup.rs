use ddnet_account_sql::version::get_version;
use ddnet_account_sql::version::set_version;
use sqlx::Acquire;
use sqlx::AnyConnection;
use sqlx::Connection;
use sqlx::Executor;
use sqlx::Statement;

const VERSION_NAME: &str = "account-game-server";

async fn setup_version1_generic(
    con: &mut AnyConnection,
    user_sql: &'static str,
) -> anyhow::Result<()> {
    // first create all statements (syntax check)
    let user = con.prepare(user_sql).await?;

    // afterwards actually create tables
    user.query().execute(&mut *con).await?;

    set_version(con, VERSION_NAME, 1).await?;

    Ok(())
}

async fn delete_generic(pool: &sqlx::AnyPool, delete_sql: &'static str) -> anyhow::Result<()> {
    let mut pool_con = pool.acquire().await?;
    let con = pool_con.acquire().await?;

    // first create all statements (syntax check)
    // delete in reverse order to creating
    let user = con.prepare(delete_sql).await?;

    // afterwards actually drop tables
    let user = user.query().execute(&mut *con).await;

    let _ = set_version(con, VERSION_NAME, 0).await;

    // handle errors at once
    user?;

    Ok(())
}

#[cfg(feature = "mysql")]
mod mysql {
    use sqlx::AnyConnection;

    pub(super) async fn setup_version1(con: &mut AnyConnection) -> anyhow::Result<()> {
        super::setup_version1_generic(con, include_str!("setup/mysql/user.sql")).await
    }

    pub(super) async fn delete(pool: &sqlx::AnyPool) -> anyhow::Result<()> {
        super::delete_generic(pool, include_str!("setup/mysql/delete/user.sql")).await
    }
}

#[cfg(feature = "sqlite")]
mod sqlite {
    use sqlx::AnyConnection;

    pub(super) async fn setup_version1(con: &mut AnyConnection) -> anyhow::Result<()> {
        super::setup_version1_generic(con, include_str!("setup/sqlite/user.sql")).await
    }

    pub(super) async fn delete(pool: &sqlx::AnyPool) -> anyhow::Result<()> {
        super::delete_generic(pool, include_str!("setup/sqlite/delete/user.sql")).await
    }
}

async fn setup_version1(con: &mut AnyConnection) -> anyhow::Result<()> {
    match con.kind() {
        #[cfg(feature = "mysql")]
        sqlx::any::AnyKind::MySql => mysql::setup_version1(con).await,
        #[cfg(feature = "sqlite")]
        sqlx::any::AnyKind::Sqlite => sqlite::setup_version1(con).await,
    }
}

/// Sets up all tables required for a game server user
pub async fn setup(pool: &sqlx::AnyPool) -> anyhow::Result<()> {
    let mut pool_con = pool.acquire().await?;
    let con = pool_con.acquire().await?;

    con.transaction(|con| {
        Box::pin(async move {
            let version = get_version(con, VERSION_NAME).await?;
            if version < 1 {
                setup_version1(&mut *con).await?;
            }

            anyhow::Ok(())
        })
    })
    .await
}

/// Drop all tables related to a game server database setup
pub async fn delete(pool: &sqlx::AnyPool) -> anyhow::Result<()> {
    match pool.any_kind() {
        #[cfg(feature = "mysql")]
        sqlx::any::AnyKind::MySql => mysql::delete(pool).await,
        #[cfg(feature = "sqlite")]
        sqlx::any::AnyKind::Sqlite => sqlite::delete(pool).await,
    }
}
