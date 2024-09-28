CREATE TABLE IF NOT EXISTS table_infra_version (
    id BIGINT NOT NULL AUTO_INCREMENT,
    -- the group name of the tables to be created
    name VARCHAR(64) NOT NULL COLLATE ascii_bin,
    version BIGINT NOT NULL,
    -- UTC timestamp! (UTC_TIMESTAMP()) of the
    -- last time this table was update
    last_update_time DATETIME NOT NULL,
    PRIMARY KEY(id),
    UNIQUE KEY(name)
);
