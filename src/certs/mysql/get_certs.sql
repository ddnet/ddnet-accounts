SELECT
    certs.cert_der
FROM
    certs
WHERE
    certs.valid_until > UTC_TIMESTAMP();
