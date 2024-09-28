INSERT INTO
    credential_auth_tokens (
        token,
        valid_until,
        ty,
        identifier,
        op
    )
VALUES
    (
        ?,
        DATE_ADD(UTC_TIMESTAMP(), INTERVAL 15 MINUTE),
        ?,
        ?,
        ?
    );
