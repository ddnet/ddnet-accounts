//! This crate contains a interfaces for common tasks
//! on the database.

#![deny(missing_docs)]
#![deny(warnings)]
#![deny(clippy::nursery)]
#![deny(clippy::all)]

use any::AnyQueryResult;

#[cfg(not(any(feature = "mysql", feature = "sqlite")))]
std::compile_error!("at least the mysql or sqlite feature must be used.");

/// Our version of sqlx any variants
pub mod any;
/// Everything related to queries
pub mod query;
/// Everything related to versioning table setups
pub mod version;

/// Checks if the query result resulted in an error that indicates
/// a duplicate entry.
pub fn is_duplicate_entry(res: &Result<AnyQueryResult, sqlx::Error>) -> bool {
    res.as_ref().is_err_and(|err| {
        if let sqlx::Error::Database(err) = err {
            matches!(err.kind(), sqlx::error::ErrorKind::UniqueViolation)
        } else {
            false
        }
    })
}
