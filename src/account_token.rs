pub mod queries;

use std::sync::Arc;

use ddnet_account_sql::query::Query;
use ddnet_accounts_shared::{
    account_server::{
        account_token::AccountTokenError, errors::AccountServerRequestError, otp::generate_otp,
        result::AccountServerReqResult,
    },
    client::account_token::{
        AccountTokenEmailRequest, AccountTokenOperation, AccountTokenSteamRequest,
    },
};
use axum::Json;
use queries::{AddAccountTokenEmail, AddAccountTokenSteam};
use sqlx::{Acquire, AnyPool};

use crate::shared::Shared;

pub async fn account_token_email(
    shared: Arc<Shared>,
    pool: AnyPool,
    requires_secret: bool,
    Json(data): Json<AccountTokenEmailRequest>,
) -> Json<AccountServerReqResult<(), AccountTokenError>> {
    // After this check a validation process could be added
    if requires_secret && data.secret_key.is_none() {
        return Json(AccountServerReqResult::Err(
            AccountServerRequestError::Other(
                "This function is only for requests with a secret verification token.".to_string(),
            ),
        ));
    }
    Json(
        account_token_email_impl(shared, pool, data)
            .await
            .map_err(|err| AccountServerRequestError::Unexpected {
                target: "account_token".into(),
                err: err.to_string(),
                bt: err.backtrace().to_string(),
            }),
    )
}

pub async fn account_token_email_impl(
    shared: Arc<Shared>,
    pool: AnyPool,
    data: AccountTokenEmailRequest,
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

    // Add a account token and send it by email
    let token = generate_otp();
    let token_hex = hex::encode(token);
    let query_add_account_token = AddAccountTokenEmail {
        token: &token,
        email: &data.email,
        ty: &data.op,
    };
    let mut connection = pool.acquire().await?;
    let con = connection.acquire().await?;

    let account_token_res = query_add_account_token
        .query(&shared.db.account_token_email_statement)
        .execute(&mut *con)
        .await?;
    anyhow::ensure!(
        account_token_res.rows_affected() >= 1,
        "No account token could be added."
    );

    let header = match data.op {
        AccountTokenOperation::LogoutAll => "DDNet Logout All Sessions",
        AccountTokenOperation::LinkCredential => "DDNet Link Credential",
        AccountTokenOperation::Delete => "DDNet Delete Account",
    };

    let mail = shared.account_tokens_email.read().clone();
    let mail = mail
        .replace("%SUBJECT%", data.email.local_part())
        .replace("%CODE%", &token_hex);
    shared
        .email
        .send_email(data.email.as_str(), header, mail)
        .await?;

    Ok(())
}

pub async fn account_token_steam(
    shared: Arc<Shared>,
    pool: AnyPool,
    requires_secret: bool,
    Json(data): Json<AccountTokenSteamRequest>,
) -> Json<AccountServerReqResult<String, AccountTokenError>> {
    // After this check a validation process could be added
    if requires_secret && data.secret_key.is_none() {
        return Json(AccountServerReqResult::Err(
            AccountServerRequestError::Other(
                "This function is only for requests with a secret verification token.".to_string(),
            ),
        ));
    }
    Json(
        account_token_steam_impl(shared, pool, data)
            .await
            .map_err(|err| AccountServerRequestError::Unexpected {
                target: "account_token".into(),
                err: err.to_string(),
                bt: err.backtrace().to_string(),
            }),
    )
}

pub async fn account_token_steam_impl(
    shared: Arc<Shared>,
    pool: AnyPool,
    data: AccountTokenSteamRequest,
) -> anyhow::Result<String> {
    anyhow::ensure!(
        data.steam_ticket.len() <= 1024,
        "Steam session auth ticket must not be bigger than 1024 bytes."
    );

    // Add a account token and send it by steam
    let token = generate_otp();
    let token_hex = hex::encode(token);
    let query_add_account_token = AddAccountTokenSteam {
        token: &token,
        steamid64: &shared.steam.verify_steamid64(data.steam_ticket).await?,
        ty: &data.op,
    };
    let mut connection = pool.acquire().await?;
    let con = connection.acquire().await?;

    let account_token_res = query_add_account_token
        .query(&shared.db.account_token_steam_statement)
        .execute(&mut *con)
        .await?;
    anyhow::ensure!(
        account_token_res.rows_affected() >= 1,
        "No account token could be added."
    );

    Ok(token_hex)
}
