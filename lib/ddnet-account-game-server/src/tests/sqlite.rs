use ddnet_account_sql::any::AnyPool;
use ddnet_accounts_shared::game_server::user_id::UserId;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

#[tokio::test]
pub async fn sqlite() -> anyhow::Result<()> {
    // ignore old test runs
    let _ = tokio::fs::remove_file(DB_FILE).await;
    const DB_FILE: &str = "test-db.sqlite";

    sqlx::any::install_default_drivers();
    let pool = AnyPool::Sqlite(
        SqlitePoolOptions::new()
            .max_connections(10)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(DB_FILE)
                    .create_if_missing(true),
            )
            .await?,
    );

    // setup
    crate::setup::setup(&pool).await?;

    let shared = crate::prepare::prepare(&pool).await?;

    // acc created
    assert!(
        crate::auto_login::auto_login(
            shared.clone(),
            &pool,
            &UserId {
                account_id: Some(0),
                public_key: Default::default(),
            },
        )
        .await?
    );

    // no acc created
    assert!(
        !crate::auto_login::auto_login(
            shared.clone(),
            &pool,
            &UserId {
                account_id: Some(0),
                public_key: Default::default(),
            },
        )
        .await?
    );

    // rename working
    crate::rename::rename(
        shared.clone(),
        &pool,
        &UserId {
            account_id: Some(0),
            public_key: Default::default(),
        },
        "my_new_name",
    )
    .await?;

    // rename exactly 32 characters
    crate::rename::rename(
        shared.clone(),
        &pool,
        &UserId {
            account_id: Some(0),
            public_key: Default::default(),
        },
        "01234567890123456789012345678901",
    )
    .await?;

    // delete
    crate::setup::delete(&pool).await?;

    tokio::fs::remove_file(DB_FILE).await?;

    Ok(())
}
