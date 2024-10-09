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
        datetime('now')
    ) ON CONFLICT(name) DO
UPDATE
SET
    version = ?,
    last_update_time = datetime('now');
