CREATE TABLE certs (
    id BIGINT NOT NULL AUTO_INCREMENT,
    cert_der BLOB,
    -- UTC timestamp! (UTC_TIMESTAMP())
    valid_until DATETIME NOT NULL,
    PRIMARY KEY(id)
);
