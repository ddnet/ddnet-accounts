use ddnet_account_sql::version::get_version;
use ddnet_account_sql::version::set_version;
use sqlx::Acquire;
use sqlx::AnyConnection;
use sqlx::Connection;
use sqlx::Executor;
use sqlx::Statement;

const VERSION_NAME: &str = "account-game-server";

async fn setup_version1_mysql(con: &mut AnyConnection) -> anyhow::Result<()> {
    // first create all statements (syntax check)
    let user = con.prepare(include_str!("setup/mysql/user.sql")).await?;

    // afterwards actually create tables
    user.query().execute(&mut *con).await?;

    set_version(con, VERSION_NAME, 1).await?;

    Ok(())
}

async fn setup_version1(con: &mut AnyConnection) -> anyhow::Result<()> {
    match con.kind() {
        sqlx::any::AnyKind::MySql => setup_version1_mysql(con).await,
    }
}

/// Sets up all mysql tables required for a game server user
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

async fn delete_mysql(pool: &sqlx::AnyPool) -> anyhow::Result<()> {
    let mut pool_con = pool.acquire().await?;
    let con = pool_con.acquire().await?;

    // first create all statements (syntax check)
    // delete in reverse order to creating
    let user = con
        .prepare(include_str!("setup/mysql/delete/user.sql"))
        .await?;

    // afterwards actually drop tables
    let user = user.query().execute(&mut *con).await;

    let _ = set_version(con, VERSION_NAME, 0).await;

    // handle errors at once
    user?;

    Ok(())
}

/// Drop all tables related to a game server mysql setup
pub async fn delete(pool: &sqlx::AnyPool) -> anyhow::Result<()> {
    match pool.any_kind() {
        sqlx::any::AnyKind::MySql => delete_mysql(pool).await,
    }
}
