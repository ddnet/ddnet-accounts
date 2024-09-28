INSERT INTO
    account_tokens (
        token,
        valid_until,
        account_id,
        ty
    )
VALUES
    (
        ?,
        DATE_ADD(UTC_TIMESTAMP(), INTERVAL 15 MINUTE),
        (
            SELECT
                id
            FROM
                account
            WHERE
                id = (
                    SELECT
                        account_id
                    FROM
                        credential_email
                    WHERE
                        email = ?
                )
        ),
        ?
    );
