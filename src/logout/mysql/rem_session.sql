DELETE FROM
    user_session
WHERE
    user_session.pub_key = ?
    AND user_session.hw_id = ?;
