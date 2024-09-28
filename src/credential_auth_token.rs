pub mod queries;

use std::sync::Arc;

use ddnet_account_sql::query::Query;
use ddnet_accounts_shared::{
    account_server::{
        credential_auth_token::CredentialAuthTokenError, errors::AccountServerRequestError,
        otp::generate_otp, result::AccountServerReqResult,
    },
    client::credential_auth_token::{
        CredentialAuthTokenEmailRequest, CredentialAuthTokenOperation,
        CredentialAuthTokenSteamRequest,
    },
};
use axum::Json;
use sqlx::{Acquire, AnyPool};

use crate::{
    credential_auth_token::queries::AddCredentialAuthToken, shared::Shared, types::TokenType,
};

pub async fn credential_auth_token_email(
    shared: Arc<Shared>,
    pool: AnyPool,
    requires_secret: bool,
    Json(data): Json<CredentialAuthTokenEmailRequest>,
) -> Json<AccountServerReqResult<(), CredentialAuthTokenError>> {
    // Check allow & deny lists
    if !shared.email.allow_list.read().is_allowed(&data.email) {
        return Json(AccountServerReqResult::Err(
            AccountServerRequestError::Other(
                "An email from that domain is not in the allowed list of email domains."
                    .to_string(),
            ),
        ));
    }
    if shared.email.deny_list.read().is_banned(&data.email) {
        return Json(AccountServerReqResult::Err(
            AccountServerRequestError::Other(
                "An email from that domain is banned and thus not allowed.".to_string(),
            ),
        ));
    }

    // Before this call a validation process could be added
    if requires_secret && data.secret_key.is_none() {
        return Json(AccountServerReqResult::Err(
            AccountServerRequestError::Other(
                "This function is only for requests with a secret verification token.".to_string(),
            ),
        ));
    }
    Json(
        credential_auth_token_email_impl(shared, pool, data)
            .await
            .map_err(|err| AccountServerRequestError::Unexpected {
                target: "credential_auth_token_email".into(),
                err: err.to_string(),
                bt: err.backtrace().to_string(),
            }),
    )
}

pub async fn credential_auth_token_email_impl(
    shared: Arc<Shared>,
    pool: AnyPool,
    data: CredentialAuthTokenEmailRequest,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        email_address::EmailAddress::parse_with_options(&data.email.email(), {
            let options = email_address::Options::default()
                .without_display_text()
                .without_domain_literal();
            if shared.email.test_mode() && data.email.domain() == "localhost" {
                options
            } else {
                options.with_required_tld()
            }
        })
        .ok()
        .as_ref()
        .map(|e| e.as_str())
            == Some(data.email.as_str()),
        "Email must only contain email part with name & domain (name@example.com)"
    );

    // write the new account to the database
    // Add a credential auth token and send it by email
    let token = generate_otp();
    let token_hex = hex::encode(token);
    let query_add_credential_auth_token = AddCredentialAuthToken {
        token: &token,
        ty: &TokenType::Email,
        identifier: data.email.as_str(),
        op: &data.op,
    };
    let mut connection = pool.acquire().await?;
    let con = connection.acquire().await?;

    let credential_auth_token_res = query_add_credential_auth_token
        .query(&shared.db.credential_auth_token_statement)
        .execute(&mut *con)
        .await?;
    anyhow::ensure!(
        credential_auth_token_res.rows_affected() >= 1,
        "No credential auth token could be added."
    );

    let header = match data.op {
        CredentialAuthTokenOperation::Login => "DDNet Account Login",
        CredentialAuthTokenOperation::LinkCredential => "DDNet Link E-mail To Account",
        CredentialAuthTokenOperation::UnlinkCredential => "DDNet Unlink Credential",
    };

    let mail = shared.credential_auth_tokens_email.read().clone();
    let mail = mail
        .replace("%SUBJECT%", data.email.local_part())
        .replace("%CODE%", &token_hex);
    shared
        .email
        .send_email(data.email.as_str(), header, mail)
        .await?;

    Ok(())
}

pub async fn credential_auth_token_steam(
    shared: Arc<Shared>,
    pool: AnyPool,
    requires_secret: bool,
    Json(data): Json<CredentialAuthTokenSteamRequest>,
) -> Json<AccountServerReqResult<String, CredentialAuthTokenError>> {
    // After this check a validation process could be added
    if requires_secret && data.secret_key.is_none() {
        return Json(AccountServerReqResult::Err(
            AccountServerRequestError::Other(
                "This function is only for requests with a secret verification token.".to_string(),
            ),
        ));
    }
    Json(
        credential_auth_token_steam_impl(shared, pool, data)
            .await
            .map_err(|err| AccountServerRequestError::Unexpected {
                target: "credential_auth_token_steam".into(),
                err: err.to_string(),
                bt: err.backtrace().to_string(),
            }),
    )
}

pub async fn credential_auth_token_steam_impl(
    shared: Arc<Shared>,
    pool: AnyPool,
    data: CredentialAuthTokenSteamRequest,
) -> anyhow::Result<String> {
    anyhow::ensure!(
        data.steam_ticket.len() <= 1024,
        "Steam session auth ticket must not be bigger than 1024 bytes."
    );

    let steamid64 = shared.steam.verify_steamid64(data.steam_ticket).await?;

    // write the new account to the database
    // Add a credential auth token and send it by steam
    let token = generate_otp();
    let token_hex = hex::encode(token);
    let query_add_credential_auth_token = AddCredentialAuthToken {
        token: &token,
        ty: &TokenType::Steam,
        identifier: &steamid64.to_string(),
        op: &data.op,
    };
    let mut connection = pool.acquire().await?;
    let con = connection.acquire().await?;

    let credential_auth_token_res = query_add_credential_auth_token
        .query(&shared.db.credential_auth_token_statement)
        .execute(&mut *con)
        .await?;
    anyhow::ensure!(
        credential_auth_token_res.rows_affected() >= 1,
        "No credential auth token could be added."
    );

    Ok(token_hex)
}
