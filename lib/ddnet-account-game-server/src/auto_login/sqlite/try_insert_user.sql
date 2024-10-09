INSERT
    OR IGNORE INTO user (
        name,
        account_id,
        create_time
    )
VALUES
    (
        ?,
        ?,
        datetime('now')
    );
