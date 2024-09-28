CREATE TABLE credential_steam (
    account_id BIGINT NOT NULL,
    steamid64 BIGINT NOT NULL,
    valid_until DATETIME NOT NULL,
    FOREIGN KEY(account_id) REFERENCES account(id),
    PRIMARY KEY(steamid64)
);
