DELETE FROM
    account_tokens
WHERE
    account_tokens.valid_until <= UTC_TIMESTAMP();
