DELETE FROM
    credential_auth_tokens
WHERE
    credential_auth_tokens.valid_until <= UTC_TIMESTAMP();
