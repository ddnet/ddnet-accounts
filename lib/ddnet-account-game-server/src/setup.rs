use ddnet_account_sql::any::AnyConnection;
use ddnet_account_sql::any::AnyPool;
use ddnet_account_sql::version::get_version;

const VERSION_NAME: &str = "account-game-server";

#[cfg(feature = "mysql")]
mod mysql {
    use ddnet_account_sql::any::AnyConnection;
    use ddnet_account_sql::version::set_version;
    use sqlx::Executor;
    use sqlx::Statement;

    use super::VERSION_NAME;

    pub(super) async fn setup_version1(
        con: &mut sqlx::mysql::MySqlConnection,
    ) -> anyhow::Result<()> {
        // first create all statements (syntax check)
        let user = con.prepare(include_str!("setup/mysql/user.sql")).await?;

        // afterwards actually create tables
        user.query().execute(&mut *con).await?;

        set_version(&mut AnyConnection::MySql(con), VERSION_NAME, 1).await?;

        Ok(())
    }

    pub(super) async fn delete(con: &mut sqlx::mysql::MySqlConnection) -> anyhow::Result<()> {
        // first create all statements (syntax check)
        // delete in reverse order to creating
        let user = con
            .prepare(include_str!("setup/mysql/delete/user.sql"))
            .await?;

        // afterwards actually drop tables
        let user = user.query().execute(&mut *con).await;

        let _ = set_version(&mut AnyConnection::MySql(con), VERSION_NAME, 0).await;

        // handle errors at once
        user?;

        Ok(())
    }
}

#[cfg(feature = "sqlite")]
mod sqlite {
    use ddnet_account_sql::any::AnyConnection;
    use ddnet_account_sql::version::set_version;
    use sqlx::Executor;
    use sqlx::Statement;

    use super::VERSION_NAME;

    pub(super) async fn setup_version1(
        con: &mut sqlx::sqlite::SqliteConnection,
    ) -> anyhow::Result<()> {
        // first create all statements (syntax check)
        let user = con.prepare(include_str!("setup/sqlite/user.sql")).await?;

        // afterwards actually create tables
        user.query().execute(&mut *con).await?;

        set_version(&mut AnyConnection::Sqlite(con), VERSION_NAME, 1).await?;

        Ok(())
    }

    pub(super) async fn delete(con: &mut sqlx::sqlite::SqliteConnection) -> anyhow::Result<()> {
        // first create all statements (syntax check)
        // delete in reverse order to creating
        let user = con
            .prepare(include_str!("setup/sqlite/delete/user.sql"))
            .await?;

        // afterwards actually drop tables
        let user = user.query().execute(&mut *con).await;

        let _ = set_version(&mut AnyConnection::Sqlite(con), VERSION_NAME, 0).await;

        // handle errors at once
        user?;

        Ok(())
    }
}

async fn setup_version1(con: &mut AnyConnection<'_>) -> anyhow::Result<()> {
    match con {
        #[cfg(feature = "mysql")]
        AnyConnection::MySql(con) => mysql::setup_version1(con).await,
        #[cfg(feature = "sqlite")]
        AnyConnection::Sqlite(con) => sqlite::setup_version1(con).await,
    }
}

/// Sets up all tables required for a game server user
pub async fn setup(pool: &AnyPool) -> anyhow::Result<()> {
    let mut pool_con = pool.acquire().await?;
    let mut con = pool_con.acquire().await?;

    con.transaction(|mut trans| {
        Box::pin(async move {
            let version = get_version(&mut trans.con(), VERSION_NAME).await?;
            if version < 1 {
                setup_version1(&mut trans.con()).await?;
            }

            anyhow::Ok(())
        })
    })
    .await
}

/// Drop all tables related to a game server database setup
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
