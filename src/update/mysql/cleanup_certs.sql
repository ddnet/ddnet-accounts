DELETE FROM
    certs
WHERE
    -- two days earlier, the corresponding keys shouldn't be in use anymore anyway
    certs.valid_until <= DATE_ADD(UTC_TIMESTAMP(), INTERVAL 2 DAY);
