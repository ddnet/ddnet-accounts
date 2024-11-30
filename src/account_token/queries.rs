use std::str::FromStr;

use anyhow::anyhow;
use async_trait::async_trait;
use ddnet_account_sql::query::Query;
use ddnet_accounts_shared::client::account_token::AccountToken;
use ddnet_accounts_types::account_id::AccountId;
use sqlx::any::AnyRow;
use sqlx::Executor;
use sqlx::Row;
use sqlx::Statement;

use crate::types::AccountTokenType;

#[derive(Debug)]
pub struct AddAccountTokenEmail<'a> {
    pub token: &'a AccountToken,
    pub email: &'a email_address::EmailAddress,
    pub ty: &'a AccountTokenType,
}

#[async_trait]
impl Query<()> for AddAccountTokenEmail<'_> {
    async fn prepare_mysql(
        connection: &mut sqlx::AnyConnection,
    ) -> anyhow::Result<sqlx::any::AnyStatement<'static>> {
        Ok(connection
            .prepare(include_str!("mysql/add_account_token_email.sql"))
            .await?)
    }
    fn query_mysql<'b>(
        &'b self,
        statement: &'b sqlx::any::AnyStatement<'static>,
    ) -> sqlx::query::Query<'b, sqlx::Any, sqlx::any::AnyArguments<'b>> {
        let ty: &'static str = self.ty.into();
        statement
            .query()
            .bind(self.token.as_slice())
            .bind(self.email.as_str())
            .bind(ty)
    }
    fn row_data(_row: &AnyRow) -> anyhow::Result<()> {
        Err(anyhow!("Row data is not supported"))
    }
}

#[derive(Debug)]
pub struct AddAccountTokenSteam<'a> {
    pub token: &'a AccountToken,
    pub steamid64: &'a i64,
    pub ty: &'a AccountTokenType,
}

#[async_trait]
impl Query<()> for AddAccountTokenSteam<'_> {
    async fn prepare_mysql(
        connection: &mut sqlx::AnyConnection,
    ) -> anyhow::Result<sqlx::any::AnyStatement<'static>> {
        Ok(connection
            .prepare(include_str!("mysql/add_account_token_steam.sql"))
            .await?)
    }
    fn query_mysql<'b>(
        &'b self,
        statement: &'b sqlx::any::AnyStatement<'static>,
    ) -> sqlx::query::Query<'b, sqlx::Any, sqlx::any::AnyArguments<'b>> {
        let ty: &'static str = self.ty.into();
        statement
            .query()
            .bind(self.token.as_slice())
            .bind(self.steamid64)
            .bind(ty)
    }
    fn row_data(_row: &AnyRow) -> anyhow::Result<()> {
        Err(anyhow!("Row data is not supported"))
    }
}

pub struct AccountTokenQry<'a> {
    pub token: &'a AccountToken,
}

pub struct AccountTokenData {
    pub account_id: AccountId,
    pub ty: AccountTokenType,
}

#[async_trait]
impl Query<AccountTokenData> for AccountTokenQry<'_> {
    async fn prepare_mysql(
        connection: &mut sqlx::AnyConnection,
    ) -> anyhow::Result<sqlx::any::AnyStatement<'static>> {
        Ok(connection
            .prepare(include_str!("mysql/account_token_data.sql"))
            .await?)
    }
    fn query_mysql<'b>(
        &'b self,
        statement: &'b sqlx::any::AnyStatement<'static>,
    ) -> sqlx::query::Query<'b, sqlx::Any, sqlx::any::AnyArguments<'b>> {
        statement.query().bind(self.token.as_slice())
    }
    fn row_data(row: &AnyRow) -> anyhow::Result<AccountTokenData> {
        Ok(AccountTokenData {
            account_id: row
                .try_get("account_id")
                .map_err(|err| anyhow!("Failed get column account_id: {err}"))?,
            ty: AccountTokenType::from_str(
                row.try_get("ty")
                    .map_err(|err| anyhow!("Failed get column ty: {err}"))?,
            )?,
        })
    }
}

pub struct InvalidateAccountToken<'a> {
    pub token: &'a AccountToken,
}

#[async_trait]
impl Query<()> for InvalidateAccountToken<'_> {
    async fn prepare_mysql(
        connection: &mut sqlx::AnyConnection,
    ) -> anyhow::Result<sqlx::any::AnyStatement<'static>> {
        Ok(connection
            .prepare(include_str!("mysql/invalidate_account_token.sql"))
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
