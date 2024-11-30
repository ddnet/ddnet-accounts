pub(crate) mod queries;

use std::sync::Arc;

use ddnet_account_sql::{any::AnyPool, query::Query};
use ddnet_accounts_shared::game_server::user_id::UserId;
use ddnet_accounts_types::account_id::AccountId;
use thiserror::Error;

use crate::shared::Shared;

use self::queries::RegisterUser;

/// The error type if registering to the game server fails.
#[derive(Error, Debug)]
pub enum AutoLoginError {
    /// A database error happened.
    #[error("{0}")]
    Database(anyhow::Error),
}

/// The prefix used for the default name generation.
pub const DEFAULT_NAME_PREFIX: &str = "autouser";

/// The default name for a given account.
pub fn default_name(account_id: &AccountId) -> String {
    format!("{DEFAULT_NAME_PREFIX}{account_id}")
}

/// Logs in the user.
///
/// Might create a new user row if the user didn't exist before.
/// Returns `true` if an account was created, which usually happens
/// if the user wasn't registered before and has a valid account id.
///
/// If the user has no account_id (account-server), then `Ok(false)` is returned.
///
/// Note: If this function returns `true`, the game server can assume
/// that the public key information in [`UserId`] belongs to this account,
/// thus it could link database entries where it only had the public key
/// information to the account now.
pub async fn auto_login(
    shared: Arc<Shared>,
    pool: &AnyPool,
    user_id: &UserId,
) -> anyhow::Result<bool, AutoLoginError> {
    if let Some(account_id) = &user_id.account_id {
        let mut pool_con = pool
            .acquire()
            .await
            .map_err(|err| AutoLoginError::Database(err.into()))?;
        let mut con = pool_con
            .acquire()
            .await
            .map_err(|err| AutoLoginError::Database(err.into()))?;

        let name = default_name(account_id);
        let qry = RegisterUser {
            account_id,
            default_name: &name,
        };

        let res = qry
            .query(&shared.db.register_user_statement)
            .execute(&mut con)
            .await
            .map_err(|err| AutoLoginError::Database(err.into()))?;

        Ok(res.rows_affected() >= 1)
    } else {
        Ok(false)
    }
}
