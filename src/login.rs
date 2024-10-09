pub mod queries;

use std::{str::FromStr, sync::Arc};

use axum::Json;
use ddnet_account_sql::query::Query;
use ddnet_accounts_shared::{
    account_server::{
        errors::AccountServerRequestError, login::LoginError, result::AccountServerReqResult,
    },
    client::login::{CredentialAuthToken, LoginRequest},
};
use ddnet_accounts_types::account_id::AccountId;
use queries::{
    AccountIdFromEmail, AccountIdFromLastInsert, AccountIdFromSteam, CredentialAuthTokenData,
    LinkAccountCredentialEmail, LinkAccountCredentialSteam,
};
use sqlx::{Acquire, AnyConnection, AnyPool, Connection};

use crate::{
    shared::Shared,
    types::{CredentialAuthTokenType, TokenType},
};

use self::queries::{
    CreateSession, CredentialAuthTokenQry, InvalidateCredentialAuthToken, TryCreateAccount,
};

pub async fn login_request(
    shared: Arc<Shared>,
    pool: AnyPool,
    Json(data): Json<LoginRequest>,
) -> Json<AccountServerReqResult<AccountId, LoginError>> {
    Json(login(shared, pool, data).await)
}

#[derive(Debug, Clone)]
enum LoginResponse {
    /// Worked
    Success(AccountId),
    /// Token invalid, probably timed out
    TokenInvalid,
}

pub async fn get_and_invalidate_credential_auth_token(
    shared: &Arc<Shared>,
    credential_auth_token: CredentialAuthToken,
    connection: &mut AnyConnection,
) -> anyhow::Result<Option<CredentialAuthTokenData>> {
    // token data
    let credential_auth_token_qry = CredentialAuthTokenQry {
        token: &credential_auth_token,
    };

    let row = credential_auth_token_qry
        .query(connection, &shared.db.credential_auth_token_qry_statement)
        .fetch_optional(connection)
        .await?;

    match row {
        Some(row) => Ok(Some(CredentialAuthTokenQry::row_data(&row)?)),
        None => Ok(None),
    }
}

pub async fn login(
    shared: Arc<Shared>,
    pool: AnyPool,
    data: LoginRequest,
) -> AccountServerReqResult<AccountId, LoginError> {
    let res = async {
        // first verify the signature
        // this step isn't really needed (security wise),
        // but at least proofs the client has a valid private key.
        data.account_data.public_key.verify_strict(
            data.credential_auth_token.as_slice(),
            &data.credential_auth_token_signature,
        )?;

        let mut connection = pool.acquire().await?;
        let connection = connection.acquire().await?;

        let res = connection
            .transaction(|connection| {
                Box::pin(async move {
                    let token_data = get_and_invalidate_credential_auth_token(
                        &shared,
                        data.credential_auth_token,
                        connection,
                    )
                    .await?;

                    let token_data = match token_data {
                        Some(token_data) => token_data,
                        None => return Ok(LoginResponse::TokenInvalid),
                    };
                    anyhow::ensure!(
                        token_data.op == CredentialAuthTokenType::Login,
                        "Credential auth token was not for loggin in for new credential"
                    );

                    enum Identifier {
                        Email(email_address::EmailAddress),
                        Steam(i64),
                    }
                    let identifier = match token_data.ty {
                        TokenType::Email => Identifier::Email(
                            email_address::EmailAddress::from_str(&token_data.identifier)?,
                        ),
                        TokenType::Steam => Identifier::Steam(token_data.identifier.parse()?),
                    };

                    // invalidate token
                    let qry = InvalidateCredentialAuthToken {
                        token: &data.credential_auth_token,
                    };
                    qry.query(
                        connection,
                        &shared.db.invalidate_credential_auth_token_statement,
                    )
                    .execute(&mut **connection)
                    .await?;

                    // create account (if not exists)
                    let account_id = match &identifier {
                        Identifier::Email(email) => {
                            // query account data
                            let qry = AccountIdFromEmail { email };

                            let row = qry
                                .query(connection, &shared.db.account_id_from_email_qry_statement)
                                .fetch_optional(&mut **connection)
                                .await?;

                            row.map(|row| AccountIdFromEmail::row_data(&row))
                                .transpose()?
                                .map(|data| data.account_id)
                        }
                        Identifier::Steam(steamid64) => {
                            // query account data
                            let qry = AccountIdFromSteam { steamid64 };

                            let row = qry
                                .query(connection, &shared.db.account_id_from_steam_qry_statement)
                                .fetch_optional(&mut **connection)
                                .await?;

                            row.map(|row| AccountIdFromSteam::row_data(&row))
                                .transpose()?
                                .map(|data| data.account_id)
                        }
                    };

                    let account_id = match account_id {
                        Some(account_id) => account_id,
                        None => {
                            let qry = TryCreateAccount {};

                            let res = qry
                                .query(connection, &shared.db.try_create_account_statement)
                                .execute(&mut **connection)
                                .await?;

                            anyhow::ensure!(res.rows_affected() >= 1, "account was not created");

                            // query account data
                            let login_qry = AccountIdFromLastInsert {};
                            let row = login_qry
                                .query(
                                    connection,
                                    &shared.db.account_id_from_last_insert_qry_statement,
                                )
                                .fetch_one(&mut **connection)
                                .await?;

                            let login_data = AccountIdFromLastInsert::row_data(&row)?;

                            match identifier {
                                Identifier::Email(email) => {
                                    let qry = LinkAccountCredentialEmail {
                                        account_id: &login_data.account_id,
                                        email: &email,
                                    };

                                    let res = qry
                                        .query(
                                            connection,
                                            &shared.db.link_credentials_email_qry_statement,
                                        )
                                        .execute(&mut **connection)
                                        .await?;

                                    anyhow::ensure!(
                                        res.rows_affected() >= 1,
                                        "account was not created, linking email failed"
                                    );
                                }
                                Identifier::Steam(steamid64) => {
                                    let qry = LinkAccountCredentialSteam {
                                        account_id: &login_data.account_id,
                                        steamid64: &steamid64,
                                    };

                                    let res = qry
                                        .query(
                                            connection,
                                            &shared.db.link_credentials_steam_qry_statement,
                                        )
                                        .execute(&mut **connection)
                                        .await?;

                                    anyhow::ensure!(
                                        res.rows_affected() >= 1,
                                        "account was not created, linking steam failed"
                                    );
                                }
                            }
                            login_data.account_id
                        }
                    };

                    let qry = CreateSession {
                        account_id,
                        hw_id: &data.account_data.hw_id,
                        pub_key: data.account_data.public_key.as_bytes(),
                    };

                    qry.query(connection, &shared.db.create_session_statement)
                        .execute(&mut **connection)
                        .await?;

                    anyhow::Ok(LoginResponse::Success(account_id))
                })
            })
            .await?;
        anyhow::Ok(res)
    }
    .await
    .map_err(|err| AccountServerRequestError::Unexpected {
        target: "login".into(),
        err: err.to_string(),
        bt: err.backtrace().to_string(),
    })?;

    match res {
        LoginResponse::Success(account_id) => Ok(account_id),
        LoginResponse::TokenInvalid => Err(AccountServerRequestError::LogicError(
            LoginError::TokenInvalid,
        )),
    }
}
