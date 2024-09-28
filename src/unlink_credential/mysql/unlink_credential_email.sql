DELETE FROM
    credential_email
WHERE
    credential_email.email = ?
    AND (
        SELECT
            COUNT(*)
        FROM
            account,
            credential_steam
        WHERE
            account.id = credential_email.account_id
            AND credential_steam.account_id = account.id
    ) > 0;
