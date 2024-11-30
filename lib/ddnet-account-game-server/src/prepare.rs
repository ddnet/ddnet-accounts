use std::sync::Arc;

use ddnet_account_sql::{any::AnyPool, query::Query};

use crate::{
    auto_login::queries::RegisterUser, db::DbConnectionShared, rename::queries::RenameUser,
    shared::Shared,
};

async fn prepare_statements(pool: &AnyPool) -> anyhow::Result<DbConnectionShared> {
    let mut pool_con = pool.acquire().await?;
    let mut con = pool_con.acquire().await?;

    Ok(DbConnectionShared {
        register_user_statement: RegisterUser::prepare(&mut con).await?,
        try_rename_statement: RenameUser::prepare(&mut con).await?,
    })
}

/// Prepare shared data used in the game server's implementation
pub async fn prepare(pool: &AnyPool) -> anyhow::Result<Arc<Shared>> {
    Ok(Arc::new(Shared {
        db: prepare_statements(pool).await?,
    }))
}
