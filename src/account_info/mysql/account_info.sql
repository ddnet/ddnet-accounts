SELECT
    account.id AS account_id,
    account.create_time AS creation_date,
    credential_email.email AS linked_email,
    credential_steam.steamid64 AS linked_steam
FROM
    account
    INNER JOIN user_session ON user_session.account_id = account.id
    LEFT JOIN credential_email ON credential_email.account_id = account.id
    LEFT JOIN credential_steam ON credential_steam.account_id = account.id
WHERE
    user_session.pub_key = ?
    AND user_session.hw_id = ?;
