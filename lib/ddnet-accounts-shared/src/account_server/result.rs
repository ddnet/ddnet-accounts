use super::errors::AccountServerRequestError;

/// The result used by all requests to the account server.
pub type AccountServerReqResult<T, E> = Result<T, AccountServerRequestError<E>>;
