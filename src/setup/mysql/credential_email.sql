CREATE TABLE credential_email (
    account_id BIGINT NOT NULL,
    email VARCHAR(255) NOT NULL,
    valid_until DATETIME NOT NULL,
    FOREIGN KEY(account_id) REFERENCES account(id),
    PRIMARY KEY(email)
);
