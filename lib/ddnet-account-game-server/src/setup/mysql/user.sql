CREATE TABLE user (
    id BIGINT NOT NULL AUTO_INCREMENT,
    name VARCHAR(32) NOT NULL COLLATE ascii_bin,
    account_id BIGINT NOT NULL,
    -- UTC timestamp! (UTC_TIMESTAMP())
    create_time DATETIME NOT NULL,
    PRIMARY KEY(id),
    UNIQUE KEY(name),
    UNIQUE KEY(account_id)
);
