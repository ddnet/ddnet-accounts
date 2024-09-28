use std::sync::Arc;

use chrono::TimeDelta;
use parking_lot::RwLock;

use crate::{
    certs::PrivateKeys, db::DbConnectionShared, email::EmailShared, ip_limit::IpDenyList,
    steam::SteamShared,
};

pub const CERT_MAX_AGE_DELTA: TimeDelta = TimeDelta::seconds(20 * 60);
pub const CERT_MIN_AGE_DELTA: TimeDelta = TimeDelta::seconds(-20 * 60);

/// Shared data across the implementation
pub struct Shared {
    pub db: DbConnectionShared,
    pub email: EmailShared,
    pub steam: SteamShared,
    /// A list of banned ips, e.g. to block VPNs
    pub ip_ban_list: Arc<RwLock<IpDenyList>>,
    /// A signing key to sign the certificates for the account users.
    pub signing_keys: Arc<RwLock<Arc<PrivateKeys>>>,
    /// All certificates that are valid for any certificate generated
    /// by any legit account server.
    pub cert_chain: Arc<RwLock<Arc<Vec<x509_cert::Certificate>>>>,
    /// The email template for credential auth tokens
    pub credential_auth_tokens_email: Arc<RwLock<Arc<String>>>,
    /// The email template for account tokens
    pub account_tokens_email: Arc<RwLock<Arc<String>>>,
}
