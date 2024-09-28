//! This crate contains a interfaces for common tasks
//! on the database.

#![deny(missing_docs)]
#![deny(warnings)]
#![deny(clippy::nursery)]
#![deny(clippy::all)]

use sqlx::{any::AnyQueryResult, Error};

/// Everything related to queries
pub mod query;
/// Everything related to versioning table setups
pub mod version;

/// Checks if the query result resulted in an error that indicates
/// a duplicate entry.
pub fn is_duplicate_entry(res: &Result<AnyQueryResult, Error>) -> bool {
    res.as_ref().is_err_and(|err| {
        if let sqlx::Error::Database(err) = err {
            [23000, 23001].contains(
                &err.code()
                    .and_then(|code| code.parse::<u32>().ok())
                    .unwrap_or_default(),
            )
        } else {
            false
        }
    })
}
