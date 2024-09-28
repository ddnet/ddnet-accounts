use crate::client::hash::argon2_hash_from_unsecure_salt;

/// A 32-byte unique id per machine.
/// On unsupported systems this creates a default id.
pub type MachineUid = [u8; 32];

/// Generates a [`MachineUid`].
/// On unsupported systems this creates a default id.
pub fn machine_uid() -> anyhow::Result<MachineUid> {
    #[cfg(not(target_os = "android"))]
    {
        argon2_hash_from_unsecure_salt(
            ::machine_uid::get()
                .map_err(|err| anyhow::anyhow!(err.to_string()))?
                .as_bytes(),
            "ddnet-hw-id".into(),
        )
    }
    #[cfg(target_os = "android")]
    argon2_hash_from_unsecure_salt(&<MachineUid>::default(), "ddnet-hw-id".into())
}
