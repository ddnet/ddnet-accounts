use crate::db::DbConnectionShared;
/// Various data that is shared for the async
/// implementations
pub struct Shared {
    /// Prepared db statements
    pub db: DbConnectionShared,
}
