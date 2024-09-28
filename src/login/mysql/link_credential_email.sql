INSERT INTO
    credential_email (account_id, email, valid_until)
VALUES
    (?, ?, DATE_ADD(UTC_TIMESTAMP(), INTERVAL 1 YEAR));
