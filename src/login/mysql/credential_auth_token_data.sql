SELECT
    credential_auth_tokens.ty,
    credential_auth_tokens.identifier,
    credential_auth_tokens.op
FROM
    credential_auth_tokens
WHERE
    credential_auth_tokens.token = ?
    AND credential_auth_tokens.valid_until > UTC_TIMESTAMP();
