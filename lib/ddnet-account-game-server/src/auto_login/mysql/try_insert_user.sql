INSERT
    IGNORE INTO user (
        name,
        account_id,
        create_time
    )
VALUES
    (
        ?,
        ?,
        UTC_TIMESTAMP()
    );
