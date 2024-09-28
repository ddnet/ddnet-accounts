CREATE TABLE account_tokens (
    account_id BIGINT NOT NULL,
    token BINARY(16) NOT NULL,
    valid_until DATETIME NOT NULL,
    -- IMPORTANT: keep with in sync with the AccountTokenType enum in src/types.rs
    ty ENUM(
        'logoutall',
        'linkcredential',
        'unlinkcredential',
        'delete'
    ) NOT NULL,
    FOREIGN KEY(account_id) REFERENCES account(id),
    PRIMARY KEY(token) USING HASH
) ENGINE = MEMORY;
