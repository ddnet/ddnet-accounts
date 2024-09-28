use anyhow::anyhow;
use argon2::{
    password_hash::{Salt, SaltString},
    Argon2, PasswordHasher,
};

/// Generates a hash for the given bytes with the given salt
/// using argon2.
///
/// # Errors
/// Only throws errors if a crypto function failed unexpected.
pub fn argon2_hash_from_salt(bytes: &[u8], salt: Salt<'_>) -> anyhow::Result<[u8; 32]> {
    // Hashed bytes salted as described above
    let argon2 = Argon2::default();
    Ok(argon2
        .hash_password(bytes, salt)
        .map_err(|err| anyhow!(err))?
        .hash
        .ok_or_else(|| anyhow!("Hash was not valid"))?
        .as_bytes()
        .try_into()?)
}

/// Generates a hash for the given bytes with the given unsecure salt
/// using argon2.
/// Should only be used to hash things that are already secure in itself.
///
/// # Errors
/// Only throws errors if a crypto function failed unexpected.
pub fn argon2_hash_from_unsecure_salt(
    bytes: &[u8],
    unsecure_salt: String,
) -> anyhow::Result<[u8; 32]> {
    argon2_hash_from_salt(
        bytes,
        SaltString::encode_b64(unsecure_salt.as_bytes())
            .map_err(|err| anyhow!(err))?
            .as_salt(),
    )
}
