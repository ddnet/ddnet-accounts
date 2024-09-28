/// All data that represents an account
/// for the client and account server.
/// This account is used to identify
/// uniquely on game-servers.
pub mod account_data;
/// Data types and operations related to prepering
/// an account info request.
pub mod account_info;
/// A data type that is used for various account related operations.
pub mod account_token;
/// Data types and operations related to getting
/// a token for credential operation.
pub mod credential_auth_token;
/// Data types and operations related to prepering
/// a delete request.
pub mod delete;
/// Create hashes using [`argon2`].
pub mod hash;
/// Data types and operations related to prepering
/// a link credential request.
pub mod link_credential;
/// Data types and operations related to prepering
/// a login request.
pub mod login;
/// Data types and operations related to prepering
/// a logout request.
pub mod logout;
/// Data types and operations related to prepering
/// a logout all request.
pub mod logout_all;
/// Get a unique identifier per machine.
/// On unsupported systems this creates a default id.
pub mod machine_id;
/// Data types and operations that the client uses
/// when an auth to the account server is issued.
pub mod sign;
/// Data types and operations related to prepering
/// a unlink credential request.
pub mod unlink_credential;
