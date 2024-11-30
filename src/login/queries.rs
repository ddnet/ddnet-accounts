use std::str::FromStr;

use anyhow::anyhow;
use axum::async_trait;
use ddnet_account_sql::query::Query;
use ddnet_accounts_shared::client::credential_auth_token::CredentialAuthTokenOperation;
use ddnet_accounts_shared::client::login::CredentialAuthToken;
use ddnet_accounts_shared::client::machine_id::MachineUid;
use ddnet_accounts_types::account_id::AccountId;
use sqlx::any::AnyRow;
use sqlx::Executor;
use sqlx::Row;
use sqlx::Statement;

use crate::types::TokenType;

pub struct CredentialAuthTokenQry<'a> {
    pub token: &'a CredentialAuthToken,
}

pub struct CredentialAuthTokenData {
    pub ty: TokenType,
    pub op: CredentialAuthTokenOperation,
    pub identifier: String,
}

#[async_trait]
impl Query<CredentialAuthTokenData> for CredentialAuthTokenQry<'_> {
    async fn prepare_mysql(
        connection: &mut sqlx::AnyConnection,
    ) -> anyhow::Result<sqlx::any::AnyStatement<'static>> {
        Ok(connection
            .prepare(include_str!("mysql/credential_auth_token_data.sql"))
            .await?)
    }
    fn query_mysql<'b>(
        &'b self,
        statement: &'b sqlx::any::AnyStatement<'static>,
    ) -> sqlx::query::Query<'b, sqlx::Any, sqlx::any::AnyArguments<'b>> {
        statement.query().bind(self.token.as_slice())
    }
    fn row_data(row: &AnyRow) -> anyhow::Result<CredentialAuthTokenData> {
        Ok(CredentialAuthTokenData {
            ty: TokenType::from_str(
                row.try_get("ty")
                    .map_err(|err| anyhow!("Failed get column ty: {err}"))?,
            )?,
            identifier: row
                .try_get("identifier")
                .map_err(|err| anyhow!("Failed get column identifier: {err}"))?,
            op: CredentialAuthTokenOperation::from_str(
                row.try_get("op")
                    .map_err(|err| anyhow!("Failed get column op: {err}"))?,
            )?,
        })
    }
}

pub struct InvalidateCredentialAuthToken<'a> {
    pub token: &'a CredentialAuthToken,
}

#[async_trait]
impl Query<()> for InvalidateCredentialAuthToken<'_> {
    async fn prepare_mysql(
        connection: &mut sqlx::AnyConnection,
    ) -> anyhow::Result<sqlx::any::AnyStatement<'static>> {
        Ok(connection
            .prepare(include_str!("mysql/invalidate_credential_auth_token.sql"))
            .await?)
    }
    fn query_mysql<'b>(
        &'b self,
        statement: &'b sqlx::any::AnyStatement<'static>,
    ) -> sqlx::query::Query<'b, sqlx::Any, sqlx::any::AnyArguments<'b>> {
        statement.query().bind(self.token.as_slice())
    }
    fn row_data(_row: &AnyRow) -> anyhow::Result<()> {
        Err(anyhow!("Row data is not supported"))
    }
}

pub struct TryCreateAccount {}

#[async_trait]
impl Query<()> for TryCreateAccount {
    async fn prepare_mysql(
        connection: &mut sqlx::AnyConnection,
    ) -> anyhow::Result<sqlx::any::AnyStatement<'static>> {
        Ok(connection
            .prepare(include_str!("mysql/add_account.sql"))
            .await?)
    }
    fn query_mysql<'b>(
        &'b self,
        statement: &'b sqlx::any::AnyStatement<'static>,
    ) -> sqlx::query::Query<'b, sqlx::Any, sqlx::any::AnyArguments<'b>> {
        statement.query()
    }
    fn row_data(_row: &AnyRow) -> anyhow::Result<()> {
        Err(anyhow!("Row data is not supported"))
    }
}

pub struct LinkAccountCredentialEmail<'a> {
    pub account_id: &'a AccountId,
    pub email: &'a email_address::EmailAddress,
}

#[async_trait]
impl Query<()> for LinkAccountCredentialEmail<'_> {
    async fn prepare_mysql(
        connection: &mut sqlx::AnyConnection,
    ) -> anyhow::Result<sqlx::any::AnyStatement<'static>> {
        Ok(connection
            .prepare(include_str!("mysql/link_credential_email.sql"))
            .await?)
    }
    fn query_mysql<'b>(
        &'b self,
        statement: &'b sqlx::any::AnyStatement<'static>,
    ) -> sqlx::query::Query<'b, sqlx::Any, sqlx::any::AnyArguments<'b>> {
        statement
            .query()
            .bind(self.account_id)
            .bind(self.email.as_str())
    }
    fn row_data(_row: &AnyRow) -> anyhow::Result<()> {
        Err(anyhow!("Row data is not supported"))
    }
}

pub struct LinkAccountCredentialSteam<'a> {
    pub account_id: &'a AccountId,
    pub steamid64: &'a i64,
}

#[async_trait]
impl Query<()> for LinkAccountCredentialSteam<'_> {
    async fn prepare_mysql(
        connection: &mut sqlx::AnyConnection,
    ) -> anyhow::Result<sqlx::any::AnyStatement<'static>> {
        Ok(connection
            .prepare(include_str!("mysql/link_credential_steam.sql"))
            .await?)
    }
    fn query_mysql<'b>(
        &'b self,
        statement: &'b sqlx::any::AnyStatement<'static>,
    ) -> sqlx::query::Query<'b, sqlx::Any, sqlx::any::AnyArguments<'b>> {
        statement.query().bind(self.account_id).bind(self.steamid64)
    }
    fn row_data(_row: &AnyRow) -> anyhow::Result<()> {
        Err(anyhow!("Row data is not supported"))
    }
}

pub struct AccountData {
    pub account_id: AccountId,
}

pub struct AccountIdFromLastInsert {}

#[async_trait]
impl Query<AccountData> for AccountIdFromLastInsert {
    async fn prepare_mysql(
        connection: &mut sqlx::AnyConnection,
    ) -> anyhow::Result<sqlx::any::AnyStatement<'static>> {
        Ok(connection
            .prepare(include_str!("mysql/account_id_from_last_insert.sql"))
            .await?)
    }
    fn query_mysql<'b>(
        &'b self,
        statement: &'b sqlx::any::AnyStatement<'static>,
    ) -> sqlx::query::Query<'b, sqlx::Any, sqlx::any::AnyArguments<'b>> {
        statement.query()
    }
    fn row_data(row: &AnyRow) -> anyhow::Result<AccountData> {
        Ok(AccountData {
            account_id: row
                .try_get("account_id")
                .map_err(|err| anyhow!("Failed get column account id: {err}"))?,
        })
    }
}

pub struct AccountIdFromEmail<'a> {
    pub email: &'a email_address::EmailAddress,
}

#[async_trait]
impl Query<AccountData> for AccountIdFromEmail<'_> {
    async fn prepare_mysql(
        connection: &mut sqlx::AnyConnection,
    ) -> anyhow::Result<sqlx::any::AnyStatement<'static>> {
        Ok(connection
            .prepare(include_str!("mysql/account_id_from_email.sql"))
            .await?)
    }
    fn query_mysql<'b>(
        &'b self,
        statement: &'b sqlx::any::AnyStatement<'static>,
    ) -> sqlx::query::Query<'b, sqlx::Any, sqlx::any::AnyArguments<'b>> {
        statement.query().bind(self.email.as_str())
    }
    fn row_data(row: &AnyRow) -> anyhow::Result<AccountData> {
        Ok(AccountData {
            account_id: row
                .try_get("account_id")
                .map_err(|err| anyhow!("Failed get column account id: {err}"))?,
        })
    }
}

pub struct AccountIdFromSteam<'a> {
    pub steamid64: &'a i64,
}

#[async_trait]
impl Query<AccountData> for AccountIdFromSteam<'_> {
    async fn prepare_mysql(
        connection: &mut sqlx::AnyConnection,
    ) -> anyhow::Result<sqlx::any::AnyStatement<'static>> {
        Ok(connection
            .prepare(include_str!("mysql/account_id_from_steam.sql"))
            .await?)
    }
    fn query_mysql<'b>(
        &'b self,
        statement: &'b sqlx::any::AnyStatement<'static>,
    ) -> sqlx::query::Query<'b, sqlx::Any, sqlx::any::AnyArguments<'b>> {
        statement.query().bind(self.steamid64)
    }
    fn row_data(row: &AnyRow) -> anyhow::Result<AccountData> {
        Ok(AccountData {
            account_id: row
                .try_get("account_id")
                .map_err(|err| anyhow!("Failed get column account id: {err}"))?,
        })
    }
}

pub struct CreateSession<'a> {
    pub account_id: AccountId,
    pub pub_key: &'a [u8; ed25519_dalek::PUBLIC_KEY_LENGTH],
    pub hw_id: &'a MachineUid,
}

#[async_trait]
impl Query<()> for CreateSession<'_> {
    async fn prepare_mysql(
        connection: &mut sqlx::AnyConnection,
    ) -> anyhow::Result<sqlx::any::AnyStatement<'static>> {
        Ok(connection
            .prepare(include_str!("mysql/add_session.sql"))
            .await?)
    }
    fn query_mysql<'b>(
        &'b self,
        statement: &'b sqlx::any::AnyStatement<'static>,
    ) -> sqlx::query::Query<'b, sqlx::Any, sqlx::any::AnyArguments<'b>> {
        statement
            .query()
            .bind(self.account_id)
            .bind(self.pub_key.as_slice())
            .bind(self.hw_id.as_slice())
    }
    fn row_data(_row: &AnyRow) -> anyhow::Result<()> {
        Err(anyhow!("Row data is not supported"))
    }
}
