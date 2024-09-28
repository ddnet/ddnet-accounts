SELECT
    user_session.account_id,
    account.create_time
FROM
    account,
    user_session
WHERE
    user_session.pub_key = ?
    AND user_session.hw_id = ?
    AND account.id = user_session.account_id;
