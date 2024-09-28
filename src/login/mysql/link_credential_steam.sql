INSERT INTO
    credential_steam (account_id, steamid64, valid_until)
VALUES
    (?, ?, DATE_ADD(UTC_TIMESTAMP(), INTERVAL 1 YEAR));
