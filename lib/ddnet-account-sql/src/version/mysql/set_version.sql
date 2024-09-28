INSERT INTO
    table_infra_version (
        name,
        version,
        last_update_time
    )
VALUES
    (
        ?,
        ?,
        UTC_TIMESTAMP()
    ) ON DUPLICATE KEY
UPDATE
    table_infra_version.version = ?,
    table_infra_version.last_update_time = UTC_TIMESTAMP();
