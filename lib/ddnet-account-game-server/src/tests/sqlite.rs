use ddnet_accounts_shared::game_server::user_id::UserId;
use sqlx::{any::AnyPoolOptions, sqlite::SqliteConnectOptions};

#[tokio::test]
pub async fn sqlite() -> anyhow::Result<()> {
    // ignore old test runs
    let _ = tokio::fs::remove_file(DB_FILE).await;
    const DB_FILE: &str = "test-db.sqlite";

    let pool = AnyPoolOptions::new()
        .max_connections(10)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(DB_FILE)
                .create_if_missing(true)
                .into(),
        )
        .await?;

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
