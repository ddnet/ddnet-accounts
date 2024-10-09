CREATE TABLE user (
    id INTEGER AUTO_INCREMENT,
    name VARCHAR(32) NOT NULL COLLATE BINARY UNIQUE,
    account_id BIGINT NOT NULL UNIQUE,
    -- UTC timestamp! (datetime('now'))
    create_time DATETIME NOT NULL,
    PRIMARY KEY(id)
);
