/// array of certificates in der format that a game server
/// can download to verify certificates for clients signed
/// by the account server.
pub type AccountServerCertificates = Vec<x509_cert::Certificate>;
