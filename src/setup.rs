use ddnet_account_sql::any::AnyConnection;
use ddnet_account_sql::any::AnyPool;
use ddnet_account_sql::version::get_version;
use ddnet_account_sql::version::set_version;
use sqlx::Executor;
use sqlx::Statement;

const VERSION_NAME: &str = "account-server";

async fn setup_version1_mysql(con: &mut sqlx::mysql::MySqlConnection) -> anyhow::Result<()> {
    // first create all statements (syntax check)
    let account = con.prepare(include_str!("setup/mysql/account.sql")).await?;
    let credential_email = con
        .prepare(include_str!("setup/mysql/credential_email.sql"))
        .await?;
    let credential_steam = con
        .prepare(include_str!("setup/mysql/credential_steam.sql"))
        .await?;
    let credential_auth_tokens = con
        .prepare(include_str!("setup/mysql/credential_auth_tokens.sql"))
        .await?;
    let account_tokens = con
        .prepare(include_str!("setup/mysql/account_tokens.sql"))
        .await?;
    let session = con.prepare(include_str!("setup/mysql/session.sql")).await?;
    let certs = con.prepare(include_str!("setup/mysql/certs.sql")).await?;

    // afterwards actually create tables
    account.query().execute(&mut *con).await?;
    credential_email.query().execute(&mut *con).await?;
    credential_steam.query().execute(&mut *con).await?;
    credential_auth_tokens.query().execute(&mut *con).await?;
    account_tokens.query().execute(&mut *con).await?;
    session.query().execute(&mut *con).await?;
    certs.query().execute(&mut *con).await?;

    set_version(&mut AnyConnection::MySql(&mut *con), VERSION_NAME, 1).await?;

    Ok(())
}

pub async fn setup_version1(con: &mut AnyConnection<'_>) -> anyhow::Result<()> {
    match con {
        AnyConnection::MySql(con) => setup_version1_mysql(con).await,
    }
}

pub async fn setup(pool: &AnyPool) -> anyhow::Result<()> {
    tokio::fs::create_dir_all("config").await?;

    let mut pool_con = pool.acquire().await?;
    let mut con = pool_con.acquire().await?;

    con.transaction(|mut con| {
        Box::pin(async move {
            let version = get_version(&mut con.con(), VERSION_NAME).await?;
            if version < 1 {
                setup_version1(&mut con.con()).await?;
            }

            anyhow::Ok(())
        })
    })
    .await
}

async fn delete_mysql(con: &mut sqlx::mysql::MySqlConnection) -> anyhow::Result<()> {
    // first create all statements (syntax check)
    // delete in reverse order to creating
    let session = con
        .prepare(include_str!("setup/mysql/delete/session.sql"))
        .await?;
    let credential_auth_tokens = con
        .prepare(include_str!(
            "setup/mysql/delete/credential_auth_tokens.sql"
        ))
        .await?;
    let account_tokens = con
        .prepare(include_str!("setup/mysql/delete/account_tokens.sql"))
        .await?;
    let credential_steam = con
        .prepare(include_str!("setup/mysql/delete/credential_steam.sql"))
        .await?;
    let credential_email = con
        .prepare(include_str!("setup/mysql/delete/credential_email.sql"))
        .await?;
    let account = con
        .prepare(include_str!("setup/mysql/delete/account.sql"))
        .await?;
    let certs = con
        .prepare(include_str!("setup/mysql/delete/certs.sql"))
        .await?;

    // afterwards actually drop tables
    let session = session.query().execute(&mut *con).await;
    let credential_auth_tokens = credential_auth_tokens.query().execute(&mut *con).await;
    let account_tokens = account_tokens.query().execute(&mut *con).await;
    let credential_steam = credential_steam.query().execute(&mut *con).await;
    let credential_email = credential_email.query().execute(&mut *con).await;
    let account = account.query().execute(&mut *con).await;
    let certs = certs.query().execute(&mut *con).await;

    let _ = set_version(&mut AnyConnection::MySql(&mut *con), VERSION_NAME, 0).await;

    // handle errors at once
    session
        .and(credential_auth_tokens)
        .and(account_tokens)
        .and(credential_steam)
        .and(credential_email)
        .and(account)
        .and(certs)?;

    Ok(())
}

pub async fn delete(pool: &AnyPool) -> anyhow::Result<()> {
    let mut con = pool.acquire().await?;
    let con = con.acquire().await?;
    match con {
        AnyConnection::MySql(con) => {
            let _ = delete_mysql(con).await;
        }
    }

    let _ = tokio::fs::remove_file("signing_keys.json").await;

    let _ = tokio::fs::remove_dir_all("config").await;

    Ok(())
}
