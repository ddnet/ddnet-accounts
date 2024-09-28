DELETE FROM
    user_session
WHERE
    user_session.account_id = ?
    AND (
        ? IS NULL
        OR (
            user_session.pub_key <> ?
            AND user_session.hw_id <> ?
        )
    );
