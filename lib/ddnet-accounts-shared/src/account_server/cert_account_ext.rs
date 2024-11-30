use const_oid::{AssociatedOid, ObjectIdentifier};
use ddnet_accounts_types::account_id::AccountId;
use serde::{Deserialize, Serialize};
use x509_cert::{
    ext::{AsExtension, Extension},
    name::Name,
};

/// The inner data type of the account extension.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, der::Sequence)]
pub struct AccountCertData {
    /// account id of the client.
    pub account_id: AccountId,
    /// The time offset to the creation date in UTC format
    /// since the UNIX epoch in milliseconds.
    pub utc_time_since_unix_epoch_millis: i64,
}

/// The x509 extension that holds the account data.
#[derive(Debug, Clone, Default, PartialEq, Eq, der::Sequence)]
pub struct AccountCertExt {
    /// actual account data, see [`AccountCertData`]
    pub data: AccountCertData,
}

impl AssociatedOid for AccountCertExt {
    /// 1.3.6.1.4.1.0 is some random valid OID.
    /// DD-Acc as ASCII code points.
    const OID: ObjectIdentifier = ObjectIdentifier::new_unwrap("1.3.6.1.4.1.0.68.68.45.65.99.99");
}

impl AsExtension for AccountCertExt {
    fn critical(&self, _subject: &Name, _extensions: &[Extension]) -> bool {
        false
    }
}
