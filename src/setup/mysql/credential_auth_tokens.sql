CREATE TABLE credential_auth_tokens (
    token BINARY(16) NOT NULL,
    valid_until DATETIME NOT NULL,
    -- IMPORTANT: keep with in sync with the TokenType enum in src/types.rs
    ty ENUM('email', 'steam') NOT NULL,
    -- IMPORTANT: keep with in sync with the AccountTokenType enum in src/types.rs
    op ENUM('login', 'linkcredential', 'unlinkcredential') NOT NULL,
    -- the email or steamid or similar depending on above type.
    identifier VARCHAR(255) NOT NULL,
    PRIMARY KEY(token) USING HASH,
    INDEX ty_identifier (ty, identifier) USING HASH,
    INDEX(identifier) USING HASH
) ENGINE = MEMORY;
