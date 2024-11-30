pub(crate) mod queries;

use std::sync::Arc;

use ddnet_account_sql::{any::AnyPool, is_duplicate_entry, query::Query};
use ddnet_accounts_shared::game_server::user_id::UserId;
use thiserror::Error;

use crate::{
    auto_login::{default_name, DEFAULT_NAME_PREFIX},
    shared::Shared,
};

use self::queries::RenameUser;

/// The error type if registering to the game server fails.
#[derive(Error, Debug)]
pub enum RenameError {
    /// A database error happened.
    #[error("{0}")]
    Database(anyhow::Error),
    /// only specific ascii characters are allowed.
    #[error("only lowercase ascii characters [a-z], [0-9], `_` are allowed.")]
    InvalidAscii,
    /// some names are not allowed.
    #[error("a user name is not allowed to start with \"autouser\".")]
    ReservedName,
    /// the user name is already taken
    #[error("a user with that name already exists.")]
    NameAlreadyExists,
    /// the user name is too short or too long
    #[error("a user must be at least 3 characters and at most 32 characters long.")]
    NameLengthInvalid,
}

/// Renames a user.
/// Returns `true` if the rename was successful.
/// Returns `false` if the user had no account.
pub async fn rename(
    shared: Arc<Shared>,
    pool: &AnyPool,
    user_id: &UserId,
    name: &str,
) -> anyhow::Result<bool, RenameError> {
    if let Some(account_id) = &user_id.account_id {
        name.chars()
            .all(|char| {
                (char.is_ascii_alphanumeric() && (char.is_ascii_lowercase() || char.is_numeric()))
                    || char == '_'
            })
            .then_some(())
            .ok_or_else(|| RenameError::InvalidAscii)?;
        let len = name.chars().count();
        (3..=32)
            .contains(&len)
            .then_some(())
            .ok_or_else(|| RenameError::NameLengthInvalid)?;
        // renaming back to the default name is allowed
        (!name.starts_with(DEFAULT_NAME_PREFIX) || name == default_name(account_id))
            .then_some(())
            .ok_or_else(|| RenameError::ReservedName)?;

        let mut pool_con = pool
            .acquire()
            .await
            .map_err(|err| RenameError::Database(err.into()))?;
        let mut con = pool_con
            .acquire()
            .await
            .map_err(|err| RenameError::Database(err.into()))?;

        let qry = RenameUser { account_id, name };

        let res = qry
            .query(&shared.db.try_rename_statement)
            .execute(&mut con)
            .await;

        if is_duplicate_entry(&res) {
            return Err(RenameError::NameAlreadyExists);
        }
        let res = res.map_err(|err| RenameError::Database(err.into()))?;

        (res.rows_affected() >= 1)
            .then_some(())
            .ok_or_else(|| RenameError::NameAlreadyExists)?;

        Ok(true)
    } else {
        Ok(false)
    }
}
