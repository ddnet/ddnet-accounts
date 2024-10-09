CREATE TABLE IF NOT EXISTS table_infra_version (
    id INTEGER AUTO_INCREMENT,
    -- the group name of the tables to be created
    name VARCHAR(64) NOT NULL COLLATE BINARY UNIQUE,
    version INTEGER NOT NULL,
    -- UTC timestamp! (datetime('now')) of the
    -- last time this table was update
    last_update_time DATETIME NOT NULL,
    PRIMARY KEY(id)
);
