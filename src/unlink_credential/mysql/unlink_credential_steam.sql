DELETE FROM
    credential_steam
WHERE
    credential_steam.steamid64 = ?
    AND (
        SELECT
            COUNT(*)
        FROM
            account,
            credential_email
        WHERE
            account.id = credential_steam.account_id
            AND credential_email.account_id = account.id
    ) > 0;
