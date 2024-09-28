use ddnet_accounts_shared::{
    account_server::{
        account_info::AccountInfoResponse, account_token::AccountTokenError,
        credential_auth_token::CredentialAuthTokenError, errors::Empty, login::LoginError,
        result::AccountServerReqResult, sign::SignResponseSuccess,
    },
    client::{
        account_data::AccountDataForClient,
        account_info::AccountInfoRequest,
        account_token::{AccountTokenEmailRequest, AccountTokenSteamRequest},
        credential_auth_token::{CredentialAuthTokenEmailRequest, CredentialAuthTokenSteamRequest},
        delete::DeleteRequest,
        link_credential::LinkCredentialRequest,
        login::LoginRequest,
        logout::LogoutRequest,
        logout_all::LogoutAllRequest,
        sign::SignRequest,
        unlink_credential::UnlinkCredentialRequest,
    },
};
use ddnet_accounts_types::account_id::AccountId;
use anyhow::anyhow;
use async_trait::async_trait;
use serde::Deserialize;

use crate::{
    errors::{FsLikeError, HttpLikeError},
    interface::Io,
};

/// Type safe version of [`Io`]
#[async_trait]
pub trait SafeIo: Sync + Send {
    async fn request_credential_auth_email_token(
        &self,
        data: CredentialAuthTokenEmailRequest,
    ) -> anyhow::Result<AccountServerReqResult<(), CredentialAuthTokenError>, HttpLikeError>;
    async fn request_credential_auth_steam_token(
        &self,
        data: CredentialAuthTokenSteamRequest,
    ) -> anyhow::Result<AccountServerReqResult<String, CredentialAuthTokenError>, HttpLikeError>;
    async fn request_credential_auth_email_token_with_secret_key(
        &self,
        data: CredentialAuthTokenEmailRequest,
    ) -> anyhow::Result<AccountServerReqResult<(), CredentialAuthTokenError>, HttpLikeError>;
    async fn request_credential_auth_steam_token_with_secret_key(
        &self,
        data: CredentialAuthTokenSteamRequest,
    ) -> anyhow::Result<AccountServerReqResult<String, CredentialAuthTokenError>, HttpLikeError>;
    async fn request_login(
        &self,
        data: LoginRequest,
    ) -> anyhow::Result<AccountServerReqResult<AccountId, LoginError>, HttpLikeError>;
    async fn request_logout(
        &self,
        data: LogoutRequest,
    ) -> anyhow::Result<AccountServerReqResult<(), Empty>, HttpLikeError>;
    async fn request_sign(
        &self,
        data: SignRequest,
    ) -> anyhow::Result<AccountServerReqResult<SignResponseSuccess, Empty>, HttpLikeError>;
    async fn request_account_token_email(
        &self,
        data: AccountTokenEmailRequest,
    ) -> anyhow::Result<AccountServerReqResult<(), AccountTokenError>, HttpLikeError>;
    async fn request_account_token_email_secret(
        &self,
        data: AccountTokenEmailRequest,
    ) -> anyhow::Result<AccountServerReqResult<(), AccountTokenError>, HttpLikeError>;
    async fn request_account_token_steam(
        &self,
        data: AccountTokenSteamRequest,
    ) -> anyhow::Result<AccountServerReqResult<String, AccountTokenError>, HttpLikeError>;
    async fn request_account_token_steam_secret(
        &self,
        data: AccountTokenSteamRequest,
    ) -> anyhow::Result<AccountServerReqResult<String, AccountTokenError>, HttpLikeError>;
    async fn request_logout_all(
        &self,
        data: LogoutAllRequest,
    ) -> anyhow::Result<AccountServerReqResult<(), Empty>, HttpLikeError>;
    async fn request_delete_account(
        &self,
        data: DeleteRequest,
    ) -> anyhow::Result<AccountServerReqResult<(), Empty>, HttpLikeError>;
    async fn request_link_credential(
        &self,
        data: LinkCredentialRequest,
    ) -> anyhow::Result<AccountServerReqResult<(), Empty>, HttpLikeError>;
    async fn request_unlink_credential(
        &self,
        data: UnlinkCredentialRequest,
    ) -> anyhow::Result<AccountServerReqResult<(), Empty>, HttpLikeError>;
    async fn request_account_info(
        &self,
        data: AccountInfoRequest,
    ) -> anyhow::Result<AccountServerReqResult<AccountInfoResponse, Empty>, HttpLikeError>;
    async fn download_account_server_certificates(
        &self,
    ) -> anyhow::Result<AccountServerReqResult<Vec<Vec<u8>>, Empty>, HttpLikeError>;
    async fn write_serialized_session_key_pair(
        &self,
        file: &AccountDataForClient,
    ) -> anyhow::Result<(), FsLikeError>;
    async fn read_serialized_session_key_pair(
        &self,
    ) -> anyhow::Result<AccountDataForClient, FsLikeError>;
    async fn remove_serialized_session_key_pair(&self) -> anyhow::Result<(), FsLikeError>;
}

pub struct IoSafe<'a> {
    pub io: &'a dyn Io,
}

impl<'a> IoSafe<'a> {
    fn des_from_vec<T>(data: Vec<u8>) -> anyhow::Result<T, HttpLikeError>
    where
        for<'de> T: Deserialize<'de>,
    {
        let s = String::from_utf8(data).map_err(|err| HttpLikeError::Other(err.into()))?;
        serde_json::from_str(s.as_str())
            .map_err(|_| HttpLikeError::Other(anyhow!("failed to parse json: {s}")))
    }
}

impl<'a> From<&'a dyn Io> for IoSafe<'a> {
    fn from(io: &'a dyn Io) -> Self {
        Self { io }
    }
}

#[async_trait]
impl<'a> SafeIo for IoSafe<'a> {
    async fn request_credential_auth_email_token(
        &self,
        data: CredentialAuthTokenEmailRequest,
    ) -> anyhow::Result<AccountServerReqResult<(), CredentialAuthTokenError>, HttpLikeError> {
        let res = self
            .io
            .request_credential_auth_email_token(serde_json::to_string(&data)?.into_bytes())
            .await?;
        Self::des_from_vec(res)
    }
    async fn request_credential_auth_steam_token(
        &self,
        data: CredentialAuthTokenSteamRequest,
    ) -> anyhow::Result<AccountServerReqResult<String, CredentialAuthTokenError>, HttpLikeError>
    {
        let res = self
            .io
            .request_credential_auth_steam_token(serde_json::to_string(&data)?.into_bytes())
            .await?;
        Self::des_from_vec(res)
    }
    async fn request_credential_auth_email_token_with_secret_key(
        &self,
        data: CredentialAuthTokenEmailRequest,
    ) -> anyhow::Result<AccountServerReqResult<(), CredentialAuthTokenError>, HttpLikeError> {
        let res = self
            .io
            .request_credential_auth_email_token_with_secret_key(
                serde_json::to_string(&data)?.into_bytes(),
            )
            .await?;
        Self::des_from_vec(res)
    }
    async fn request_credential_auth_steam_token_with_secret_key(
        &self,
        data: CredentialAuthTokenSteamRequest,
    ) -> anyhow::Result<AccountServerReqResult<String, CredentialAuthTokenError>, HttpLikeError>
    {
        let res = self
            .io
            .request_credential_auth_steam_token_with_secret_key(
                serde_json::to_string(&data)?.into_bytes(),
            )
            .await?;
        Self::des_from_vec(res)
    }
    async fn request_login(
        &self,
        data: LoginRequest,
    ) -> anyhow::Result<AccountServerReqResult<AccountId, LoginError>, HttpLikeError> {
        let res = self
            .io
            .request_login(serde_json::to_string(&data)?.into_bytes())
            .await?;
        Self::des_from_vec(res)
    }
    async fn request_logout(
        &self,
        data: LogoutRequest,
    ) -> anyhow::Result<AccountServerReqResult<(), Empty>, HttpLikeError> {
        let res = self
            .io
            .request_logout(serde_json::to_string(&data)?.into_bytes())
            .await?;

        Self::des_from_vec(res)
    }
    async fn request_sign(
        &self,
        data: SignRequest,
    ) -> anyhow::Result<AccountServerReqResult<SignResponseSuccess, Empty>, HttpLikeError> {
        let res = self
            .io
            .request_sign(serde_json::to_string(&data)?.into_bytes())
            .await?;
        Self::des_from_vec(res)
    }
    async fn request_account_token_email(
        &self,
        data: AccountTokenEmailRequest,
    ) -> anyhow::Result<AccountServerReqResult<(), AccountTokenError>, HttpLikeError> {
        let res = self
            .io
            .request_account_token_email(serde_json::to_string(&data)?.into_bytes())
            .await?;
        Self::des_from_vec(res)
    }
    async fn request_account_token_email_secret(
        &self,
        data: AccountTokenEmailRequest,
    ) -> anyhow::Result<AccountServerReqResult<(), AccountTokenError>, HttpLikeError> {
        let res = self
            .io
            .request_account_token_email_secret(serde_json::to_string(&data)?.into_bytes())
            .await?;
        Self::des_from_vec(res)
    }
    async fn request_account_token_steam(
        &self,
        data: AccountTokenSteamRequest,
    ) -> anyhow::Result<AccountServerReqResult<String, AccountTokenError>, HttpLikeError> {
        let res = self
            .io
            .request_account_token_steam(serde_json::to_string(&data)?.into_bytes())
            .await?;
        Self::des_from_vec(res)
    }
    async fn request_account_token_steam_secret(
        &self,
        data: AccountTokenSteamRequest,
    ) -> anyhow::Result<AccountServerReqResult<String, AccountTokenError>, HttpLikeError> {
        let res = self
            .io
            .request_account_token_steam_secret(serde_json::to_string(&data)?.into_bytes())
            .await?;
        Self::des_from_vec(res)
    }
    async fn request_logout_all(
        &self,
        data: LogoutAllRequest,
    ) -> anyhow::Result<AccountServerReqResult<(), Empty>, HttpLikeError> {
        let res = self
            .io
            .request_logout_all(serde_json::to_string(&data)?.into_bytes())
            .await?;
        Self::des_from_vec(res)
    }
    async fn request_delete_account(
        &self,
        data: DeleteRequest,
    ) -> anyhow::Result<AccountServerReqResult<(), Empty>, HttpLikeError> {
        let res = self
            .io
            .request_delete_account(serde_json::to_string(&data)?.into_bytes())
            .await?;
        Self::des_from_vec(res)
    }
    async fn request_link_credential(
        &self,
        data: LinkCredentialRequest,
    ) -> anyhow::Result<AccountServerReqResult<(), Empty>, HttpLikeError> {
        let res = self
            .io
            .request_link_credential(serde_json::to_string(&data)?.into_bytes())
            .await?;
        Self::des_from_vec(res)
    }
    async fn request_unlink_credential(
        &self,
        data: UnlinkCredentialRequest,
    ) -> anyhow::Result<AccountServerReqResult<(), Empty>, HttpLikeError> {
        let res = self
            .io
            .request_unlink_credential(serde_json::to_string(&data)?.into_bytes())
            .await?;
        Self::des_from_vec(res)
    }
    async fn request_account_info(
        &self,
        data: AccountInfoRequest,
    ) -> anyhow::Result<AccountServerReqResult<AccountInfoResponse, Empty>, HttpLikeError> {
        let res = self
            .io
            .request_account_info(serde_json::to_string(&data)?.into_bytes())
            .await?;
        Self::des_from_vec(res)
    }
    async fn download_account_server_certificates(
        &self,
    ) -> anyhow::Result<AccountServerReqResult<Vec<Vec<u8>>, Empty>, HttpLikeError> {
        let res = self.io.download_account_server_certificates().await?;

        Self::des_from_vec(res)
    }
    async fn write_serialized_session_key_pair(
        &self,
        file: &AccountDataForClient,
    ) -> anyhow::Result<(), FsLikeError> {
        self.io
            .write_serialized_session_key_pair(
                serde_json::to_string(file)
                    .map_err(|err| FsLikeError::Other(err.into()))?
                    .into_bytes(),
            )
            .await
    }
    async fn read_serialized_session_key_pair(
        &self,
    ) -> anyhow::Result<AccountDataForClient, FsLikeError> {
        Ok(
            serde_json::from_slice(&self.io.read_serialized_session_key_pair().await?)
                .map_err(|err| FsLikeError::Other(err.into()))?,
        )
    }
    async fn remove_serialized_session_key_pair(&self) -> anyhow::Result<(), FsLikeError> {
        self.io.remove_serialized_session_key_pair().await
    }
}
