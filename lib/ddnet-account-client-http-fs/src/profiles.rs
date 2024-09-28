use std::{
    collections::HashMap,
    fmt::Debug,
    future::Future,
    ops::Deref,
    path::PathBuf,
    pin::Pin,
    sync::Arc,
    time::{Duration, SystemTime},
};

use anyhow::anyhow;
pub use ddnet_account_client::{
    account_token::AccountTokenResult, credential_auth_token::CredentialAuthTokenResult,
};
use ddnet_account_client::{interface::Io, sign::SignResult};
use ddnet_accounts_shared::{
    account_server::account_info::AccountInfoResponse,
    cert::generate_self_signed,
    client::{
        account_data::{key_pair, AccountDataForClient},
        account_token::AccountTokenOperation,
        credential_auth_token::CredentialAuthTokenOperation,
    },
};
use ddnet_accounts_types::account_id::AccountId;
use either::Either;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use x509_cert::der::Decode;

pub use x509_cert::Certificate;

use crate::{client::DeleteAccountExt, fs::Fs};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ProfileData {
    pub name: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct ProfilesState {
    pub profiles: HashMap<String, ProfileData>,
    pub cur_profile: String,
}

impl ProfilesState {
    async fn load_or_default(fs: &Fs) -> Self {
        fs.read("profiles.json".as_ref())
            .await
            .map_err(|err| anyhow!(err))
            .and_then(|file| serde_json::from_slice(&file).map_err(|err| anyhow!(err)))
            .unwrap_or_default()
    }

    async fn save(&self, fs: &Fs) -> anyhow::Result<()> {
        let file_content = serde_json::to_vec_pretty(self)?;
        fs.write("".as_ref(), "profiles.json".as_ref(), file_content)
            .await?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct ProfileCertAndKeys {
    pub cert: Certificate,
    pub key_pair: AccountDataForClient,
    pub valid_duration: Duration,
}

#[derive(Debug, Default, Clone)]
pub enum ProfileCert {
    #[default]
    None,
    Fetching(Arc<tokio::sync::Notify>),
    CertAndKeys(Box<ProfileCertAndKeys>),
    CertAndKeysAndFetch {
        cert_and_keys: Box<ProfileCertAndKeys>,
        notifier: Arc<tokio::sync::Notify>,
    },
}

#[derive(Debug)]
pub struct ActiveProfile<C: Io + DeleteAccountExt + Debug> {
    client: Arc<C>,
    cur_cert: Arc<Mutex<ProfileCert>>,

    profile_data: ProfileData,
}

#[derive(Debug, Default)]
pub struct ActiveProfiles<C: Io + DeleteAccountExt + Debug> {
    profiles: HashMap<String, ActiveProfile<C>>,
    cur_profile: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AccountlessKeysAndValidy {
    account_data: AccountDataForClient,
    valid_until: chrono::DateTime<chrono::Utc>,
}
// 3 months validy
fn accountless_validy_range() -> Duration {
    Duration::from_secs(60 * 60 * 24 * 30 * 3)
}
const ACCOUNTLESS_KEYS_FILE: &str = "accountless_keys_and_cert.json";

/// Helper for multiple account profiles.
#[derive(Debug)]
pub struct Profiles<
    C: Io + DeleteAccountExt + Debug,
    F: Deref<
            Target = dyn Fn(
                PathBuf,
            )
                -> Pin<Box<dyn Future<Output = anyhow::Result<C>> + Sync + Send>>,
        > + Debug
        + Sync
        + Send,
> {
    profiles: Arc<parking_lot::Mutex<ActiveProfiles<C>>>,
    factory: Arc<F>,
    secure_base_path: Arc<PathBuf>,
    fs: Fs,
}

impl<
        C: Io + DeleteAccountExt + Debug + 'static,
        F: Deref<
                Target = dyn Fn(
                    PathBuf,
                )
                    -> Pin<Box<dyn Future<Output = anyhow::Result<C>> + Sync + Send>>,
            > + Debug
            + Sync
            + Send,
    > Profiles<C, F>
{
    fn to_profile_states(profiles: &ActiveProfiles<C>) -> ProfilesState {
        let mut res = ProfilesState::default();

        res.profiles.extend(
            profiles
                .profiles
                .iter()
                .map(|(key, val)| (key.clone(), val.profile_data.clone())),
        );
        res.cur_profile.clone_from(&profiles.cur_profile);

        res
    }

    fn account_id_to_path(account_id: AccountId) -> String {
        format!("acc_{}", account_id)
    }

    pub fn new(loading: ProfilesLoading<C, F>) -> Self {
        Self {
            profiles: Arc::new(loading.profiles),
            factory: loading.factory,
            secure_base_path: Arc::new(loading.secure_base_path),
            fs: loading.fs,
        }
    }

    /// generate a token for a new email credential auth attempt.
    pub async fn credential_auth_email_token(
        &self,
        email: email_address::EmailAddress,
        op: CredentialAuthTokenOperation,
        secret_key_hex: Option<String>,
    ) -> anyhow::Result<(), CredentialAuthTokenResult> {
        let path = self.secure_base_path.join("acc_prepare");
        let account_client = Arc::new(
            (self.factory)(path)
                .await
                .map_err(CredentialAuthTokenResult::Other)?,
        );

        ddnet_account_client::credential_auth_token::credential_auth_token_email(
            email,
            op,
            secret_key_hex,
            account_client.as_ref(),
        )
        .await?;

        Ok(())
    }

    /// generate a token for a new steam credential auth attempt.
    pub async fn credential_auth_steam_token(
        &self,
        steam_ticket: Vec<u8>,
        op: CredentialAuthTokenOperation,
        secret_key_hex: Option<String>,
    ) -> anyhow::Result<String, CredentialAuthTokenResult> {
        let path = self.secure_base_path.join("acc_prepare");
        let account_client = Arc::new(
            (self.factory)(path)
                .await
                .map_err(CredentialAuthTokenResult::Other)?,
        );

        ddnet_account_client::credential_auth_token::credential_auth_token_steam(
            steam_ticket,
            op,
            secret_key_hex,
            account_client.as_ref(),
        )
        .await
    }

    /// generate a token for a new email account operation attempt.
    pub async fn account_email_token(
        &self,
        email: email_address::EmailAddress,
        op: AccountTokenOperation,
        secret_key_hex: Option<String>,
    ) -> anyhow::Result<(), AccountTokenResult> {
        let path = self.secure_base_path.join("acc_prepare");
        let account_client = Arc::new(
            (self.factory)(path)
                .await
                .map_err(AccountTokenResult::Other)?,
        );

        ddnet_account_client::account_token::account_token_email(
            email,
            op,
            secret_key_hex,
            account_client.as_ref(),
        )
        .await?;

        Ok(())
    }

    /// generate a token for a new steam account operation attempt.
    pub async fn account_steam_token(
        &self,
        steam_ticket: Vec<u8>,
        op: AccountTokenOperation,
        secret_key_hex: Option<String>,
    ) -> anyhow::Result<String, AccountTokenResult> {
        let path = self.secure_base_path.join("acc_prepare");
        let account_client = Arc::new(
            (self.factory)(path)
                .await
                .map_err(AccountTokenResult::Other)?,
        );

        ddnet_account_client::account_token::account_token_steam(
            steam_ticket,
            op,
            secret_key_hex,
            account_client.as_ref(),
        )
        .await
    }

    async fn read_accountless_keys(fs: &Fs) -> anyhow::Result<AccountlessKeysAndValidy> {
        fs.read(ACCOUNTLESS_KEYS_FILE.as_ref())
            .await
            .map_err(|err| anyhow!(err))
            .and_then(|file| {
                serde_json::from_slice::<AccountlessKeysAndValidy>(&file)
                    .map_err(|err| anyhow!(err))
            })
            .and_then(|accountless_keys_and_validy| {
                let now: chrono::DateTime<chrono::Utc> = std::time::SystemTime::now().into();
                (now.signed_duration_since(accountless_keys_and_validy.valid_until)
                    < chrono::TimeDelta::new(
                        accountless_validy_range().as_secs() as i64,
                        accountless_validy_range().subsec_nanos(),
                    )
                    .unwrap_or(chrono::TimeDelta::max_value()))
                .then_some(accountless_keys_and_validy)
                .ok_or_else(|| anyhow!("accountless keys too old"))
            })
    }

    async fn take_accountless_keys(&self) -> anyhow::Result<AccountDataForClient> {
        let account_data = Self::read_accountless_keys(&self.fs).await?;

        self.fs.remove(ACCOUNTLESS_KEYS_FILE.as_ref()).await?;

        Ok(account_data.account_data)
    }

    async fn login_impl(
        &self,
        display_name: &str,
        credential_auth_token_hex: String,
    ) -> anyhow::Result<()> {
        let path = self.secure_base_path.join("acc_prepare");
        let account_client = Arc::new((self.factory)(path).await?);

        // first try to "upgrade" the accountless keys to a real account.
        let (account_id, login_data_writer) = if let Ok(account_data) =
            self.take_accountless_keys().await
        {
            ddnet_account_client::login::login_with_account_data(
                credential_auth_token_hex,
                &account_data,
                account_client.as_ref(),
            )
            .await?
        } else {
            ddnet_account_client::login::login(credential_auth_token_hex, account_client.as_ref())
                .await?
        };

        let profile_name = Self::account_id_to_path(account_id);
        let path = self.secure_base_path.join(&profile_name);
        let account_client = Arc::new((self.factory)(path).await?);

        login_data_writer.write(&*account_client).await?;

        let profile = ActiveProfile {
            client: account_client,
            cur_cert: Default::default(),
            profile_data: ProfileData {
                name: display_name.to_string(),
            },
        };

        let profiles_state;
        {
            let mut profiles = self.profiles.lock();
            profiles.profiles.insert(profile_name.to_string(), profile);
            profiles.cur_profile = profile_name.to_string();
            profiles_state = Self::to_profile_states(&profiles);
            drop(profiles);
        }

        profiles_state.save(&self.fs).await?;

        self.signed_cert_and_key_pair().await;

        Ok(())
    }

    /// try to login via credential auth token previously created with e.g. [`Self::credential_auth_email_token`]
    pub async fn login_email(
        &self,
        email: email_address::EmailAddress,
        credential_auth_token_hex: String,
    ) -> anyhow::Result<()> {
        self.login_impl(
            &format!("{}'s account", email.local_part()),
            credential_auth_token_hex,
        )
        .await
    }

    /// try to login via credential auth token previously created with e.g. [`Self::login_steam_token`]
    pub async fn login_steam(
        &self,
        steam_user_name: String,
        credential_auth_token_hex: String,
    ) -> anyhow::Result<()> {
        self.login_impl(
            &format!("{}'s account", steam_user_name),
            credential_auth_token_hex,
        )
        .await
    }

    /// removes the profile
    async fn remove_profile(
        profiles: Arc<parking_lot::Mutex<ActiveProfiles<C>>>,
        fs: &Fs,
        profile_name: &str,
    ) -> anyhow::Result<()> {
        let profiles_state;
        let removed_profile;
        {
            let mut profiles = profiles.lock();
            removed_profile = profiles.profiles.remove(profile_name);
            if profiles.cur_profile == profile_name {
                profiles.cur_profile = profiles.profiles.keys().next().cloned().unwrap_or_default();
            }
            profiles_state = Self::to_profile_states(&profiles);
            drop(profiles);
        }

        profiles_state.save(fs).await?;

        if let Some(profile) = removed_profile {
            let _ = profile.client.remove_account().await;
        }

        Ok(())
    }

    /// If no account was found, fall back to key-pair that
    /// is not account based, but could be upgraded
    async fn account_less_cert_and_key_pair(
        fs_or_account_data: Either<&Fs, AccountDataForClient>,
        err: Option<anyhow::Error>,
    ) -> (AccountDataForClient, Certificate, Option<anyhow::Error>) {
        match fs_or_account_data {
            Either::Left(fs) => {
                let (account_data, cert) = if let Ok((account_data, cert)) =
                    Self::read_accountless_keys(fs)
                        .await
                        .and_then(|accountless_keys_and_validy| {
                            generate_self_signed(
                                &accountless_keys_and_validy.account_data.private_key,
                            )
                            .map_err(|err| anyhow!(err))
                            .map(|cert| (accountless_keys_and_validy.account_data, cert))
                        }) {
                    (account_data, cert)
                } else {
                    let (private_key, public_key) = key_pair();

                    let cert = generate_self_signed(&private_key).unwrap();

                    // save the newely generated cert & account data
                    let accountless_keys_and_cert = AccountlessKeysAndValidy {
                        account_data: AccountDataForClient {
                            private_key,
                            public_key,
                        },
                        valid_until: (std::time::SystemTime::now() + accountless_validy_range())
                            .into(),
                    };

                    // ignore errors, can't recover anyway
                    if let Ok(file) = serde_json::to_vec(&accountless_keys_and_cert) {
                        let _ = fs
                            .write("".as_ref(), ACCOUNTLESS_KEYS_FILE.as_ref(), file)
                            .await;
                    }

                    (accountless_keys_and_cert.account_data, cert)
                };
                (account_data, cert, err)
            }
            Either::Right(account_data) => {
                let cert = generate_self_signed(&account_data.private_key).unwrap();
                (account_data, cert, err)
            }
        }
    }

    /// Gets a _recently_ signed cerificate from the accounts server
    /// and the key pair of the client.
    /// If an error occurred a self signed cert & key-pair will still be generated to
    /// allow playing at all cost.
    /// It's up to the implementation how it wants to inform the user about
    /// this error.
    pub async fn signed_cert_and_key_pair(
        &self,
    ) -> (AccountDataForClient, Certificate, Option<anyhow::Error>) {
        let mut cur_cert_der = None;
        let mut account_client = None;
        let mut cur_profile = None;
        {
            let profiles = self.profiles.lock();
            if let Some(profile) = profiles.profiles.get(&profiles.cur_profile) {
                cur_cert_der = Some(profile.cur_cert.clone());
                account_client = Some(profile.client.clone());
                cur_profile = Some(profiles.cur_profile.clone());
            }
            drop(profiles);
        }

        if let Some(((cur_cert, client), cur_profile)) =
            cur_cert_der.zip(account_client).zip(cur_profile)
        {
            let mut try_fetch = None;
            let mut try_wait = None;
            {
                let mut cert = cur_cert.lock();
                match &*cert {
                    ProfileCert::None => {
                        let notifier: Arc<tokio::sync::Notify> = Default::default();
                        *cert = ProfileCert::Fetching(notifier.clone());
                        try_fetch = Some((notifier, true));
                    }
                    ProfileCert::Fetching(notifier) => {
                        try_wait = Some(notifier.clone());
                    }
                    ProfileCert::CertAndKeys(cert_and_keys) => {
                        // check if cert is outdated
                        let expires_at = cert_and_keys
                            .cert
                            .tbs_certificate
                            .validity
                            .not_after
                            .to_system_time();
                        // if it is about to expire, fetch again replacing the old ones
                        if expires_at < SystemTime::now() + Duration::from_secs(60 * 10) {
                            let notifier: Arc<tokio::sync::Notify> = Default::default();
                            *cert = ProfileCert::Fetching(notifier.clone());
                            try_fetch = Some((notifier, true));
                        }
                        // else if the cert's lifetime already hit the half, try to fetch, but don't replace the existing one
                        else if expires_at < SystemTime::now() + cert_and_keys.valid_duration / 2
                        {
                            let notifier: Arc<tokio::sync::Notify> = Default::default();
                            *cert = ProfileCert::CertAndKeysAndFetch {
                                cert_and_keys: cert_and_keys.clone(),
                                notifier: notifier.clone(),
                            };
                            try_fetch = Some((notifier, false));
                        }
                    }
                    ProfileCert::CertAndKeysAndFetch {
                        cert_and_keys,
                        notifier,
                    } => {
                        // if fetching gets urgent, downgrade this to fetch operation
                        let expires_at = cert_and_keys
                            .cert
                            .tbs_certificate
                            .validity
                            .not_after
                            .to_system_time();
                        if expires_at < SystemTime::now() + Duration::from_secs(60 * 10) {
                            let notifier = notifier.clone();
                            *cert = ProfileCert::Fetching(notifier.clone());
                            try_wait = Some(notifier);
                        }
                        // else just ignore
                    }
                }
            }

            if let Some(notifier) = try_wait {
                notifier.notified().await;
                // notify the next one
                notifier.notify_one();
            }

            let should_wait = if let Some((notifier, should_wait)) = try_fetch {
                let fs = self.fs.clone();
                let profiles = self.profiles.clone();
                let cur_cert = cur_cert.clone();
                let res = tokio::spawn(async move {
                    let res = match ddnet_account_client::sign::sign(client.as_ref()).await {
                        Ok(sign_data) => {
                            if let Ok(cert) = Certificate::from_der(&sign_data.certificate_der) {
                                *cur_cert.lock() =
                                    ProfileCert::CertAndKeys(Box::new(ProfileCertAndKeys {
                                        cert: cert.clone(),
                                        key_pair: sign_data.session_key_pair.clone(),
                                        valid_duration: cert
                                            .tbs_certificate
                                            .validity
                                            .not_after
                                            .to_system_time()
                                            .duration_since(SystemTime::now())
                                            .unwrap_or(Duration::ZERO),
                                    }));
                                (sign_data.session_key_pair, cert, None)
                            } else {
                                Self::account_less_cert_and_key_pair(
                                    Either::Left(&fs),
                                    Some(anyhow!(
                                        "account server did not return a valid certificate, \
                                        please contact a developer."
                                    )),
                                )
                                .await
                            }
                        }
                        Err(err) => {
                            *cur_cert.lock() = ProfileCert::None;
                            // if the error was a file system error
                            // or session was invalid for other reasons, then remove that profile.
                            match err {
                                SignResult::SessionWasInvalid | SignResult::FsLikeError(_) => {
                                    // try to remove that profile
                                    let _ = Self::remove_profile(profiles, &fs, &cur_profile).await;
                                    Self::account_less_cert_and_key_pair(
                                        Either::Left(&fs),
                                        Some(err.into()),
                                    )
                                    .await
                                }
                                SignResult::HttpLikeError {
                                    ref account_data, ..
                                }
                                | SignResult::Other {
                                    ref account_data, ..
                                } => {
                                    // tell the fallback key mechanism to try the account data,
                                    // even if self signed, this can allow a game server
                                    // to recover lost account related data. (But does not require to)
                                    Self::account_less_cert_and_key_pair(
                                        Either::Right(account_data.clone()),
                                        Some(err.into()),
                                    )
                                    .await
                                }
                            }
                        }
                    };
                    notifier.notify_one();
                    res
                });
                should_wait.then_some(res)
            } else {
                None
            };

            // if fetching was urgent, it must wait for the task to complete.
            let awaited_task = if let Some(task) = should_wait {
                task.await.ok()
            } else {
                None
            };

            if let Some(res) = awaited_task {
                res
            } else {
                let (ProfileCert::CertAndKeys(cert_and_keys)
                | ProfileCert::CertAndKeysAndFetch { cert_and_keys, .. }) = cur_cert.lock().clone()
                else {
                    return Self::account_less_cert_and_key_pair(
                        Either::Left(&self.fs),
                        Some(anyhow!("no cert or key found.")),
                    )
                    .await;
                };
                let ProfileCertAndKeys { cert, key_pair, .. } = *cert_and_keys;

                (key_pair, cert, None)
            }
        } else {
            Self::account_less_cert_and_key_pair(Either::Left(&self.fs), None).await
        }
    }

    /// Tries to logout the given profile
    pub async fn logout(&self, profile_name: &str) -> anyhow::Result<()> {
        let mut account_client = None;
        {
            let profiles = self.profiles.lock();
            if let Some(profile) = profiles.profiles.get(profile_name) {
                account_client = Some(profile.client.clone());
            }
            drop(profiles);
        }
        let Some(account_client) = account_client else {
            return Err(anyhow::anyhow!(
                "Profile with name {} not found",
                profile_name
            ));
        };
        ddnet_account_client::logout::logout(&*account_client).await?;
        Self::remove_profile(self.profiles.clone(), &self.fs, profile_name).await
    }

    /// Tries to logout all session except the current for the given profile
    pub async fn logout_all(
        &self,
        account_token_hex: String,
        profile_name: &str,
    ) -> anyhow::Result<()> {
        let mut account_client = None;
        {
            let profiles = self.profiles.lock();
            if let Some(profile) = profiles.profiles.get(profile_name) {
                account_client = Some(profile.client.clone());
            }
            drop(profiles);
        }
        let Some(account_client) = account_client else {
            return Err(anyhow::anyhow!(
                "Profile with name {} not found",
                profile_name
            ));
        };
        Ok(
            ddnet_account_client::logout_all::logout_all(account_token_hex, &*account_client)
                .await?,
        )
    }

    /// Tries to delete the account of the given profile
    pub async fn delete(
        &self,
        account_token_hex: String,
        profile_name: &str,
    ) -> anyhow::Result<()> {
        let mut account_client = None;
        {
            let profiles = self.profiles.lock();
            if let Some(profile) = profiles.profiles.get(profile_name) {
                account_client = Some(profile.client.clone());
            }
            drop(profiles);
        }
        let Some(account_client) = account_client else {
            return Err(anyhow::anyhow!(
                "Profile with name {} not found",
                profile_name
            ));
        };
        ddnet_account_client::delete::delete(account_token_hex, &*account_client).await?;
        Self::remove_profile(self.profiles.clone(), &self.fs, profile_name).await
    }

    /// Tries to link a credential for the given profile
    pub async fn link_credential(
        &self,
        account_token_hex: String,
        credential_auth_token_hex: String,
        profile_name: &str,
    ) -> anyhow::Result<()> {
        let mut account_client = None;
        {
            let profiles = self.profiles.lock();
            if let Some(profile) = profiles.profiles.get(profile_name) {
                account_client = Some(profile.client.clone());
            }
            drop(profiles);
        }
        let Some(account_client) = account_client else {
            return Err(anyhow::anyhow!(
                "Profile with name {} not found",
                profile_name
            ));
        };
        Ok(ddnet_account_client::link_credential::link_credential(
            account_token_hex,
            credential_auth_token_hex,
            &*account_client,
        )
        .await?)
    }

    /// Tries to unlink a credential for the given profile
    pub async fn unlink_credential(
        &self,
        credential_auth_token_hex: String,
        profile_name: &str,
    ) -> anyhow::Result<()> {
        let mut account_client = None;
        {
            let profiles = self.profiles.lock();
            if let Some(profile) = profiles.profiles.get(profile_name) {
                account_client = Some(profile.client.clone());
            }
            drop(profiles);
        }
        let Some(account_client) = account_client else {
            return Err(anyhow::anyhow!(
                "Profile with name {} not found",
                profile_name
            ));
        };
        Ok(ddnet_account_client::unlink_credential::unlink_credential(
            credential_auth_token_hex,
            &*account_client,
        )
        .await?)
    }

    /// Tries to fetch the account info for the given profile
    pub async fn account_info(&self, profile_name: &str) -> anyhow::Result<AccountInfoResponse> {
        let mut account_client = None;
        {
            let profiles = self.profiles.lock();
            if let Some(profile) = profiles.profiles.get(profile_name) {
                account_client = Some(profile.client.clone());
            }
            drop(profiles);
        }
        let Some(account_client) = account_client else {
            return Err(anyhow::anyhow!(
                "Profile with name {} not found",
                profile_name
            ));
        };
        Ok(ddnet_account_client::account_info::account_info(&*account_client).await?)
    }

    /// Currently loaded profiles
    pub fn profiles(&self) -> (HashMap<String, ProfileData>, String) {
        let profiles = self.profiles.lock();
        let profiles = Self::to_profile_states(&profiles);
        (profiles.profiles, profiles.cur_profile)
    }

    /// Set the current profile to a new one.
    /// Silently fails, if the new profile does not exist.
    pub async fn set_profile(&self, profile_name: &str) {
        let profiles_state;
        {
            let mut profiles = self.profiles.lock();
            if profiles.profiles.contains_key(profile_name) {
                profiles.cur_profile = profile_name.to_string();
            }
            profiles_state = Self::to_profile_states(&profiles);
            drop(profiles);
        }

        let _ = profiles_state.save(&self.fs).await;
    }

    /// Set the profile's display name to a new one.
    /// Silently fails, if the profile does not exist.
    pub async fn set_profile_display_name(&self, profile_name: &str, display_name: String) {
        let profiles_state;
        {
            let mut profiles = self.profiles.lock();
            if let Some(profile) = profiles.profiles.get_mut(profile_name) {
                profile.profile_data.name = display_name;
            }
            profiles_state = Self::to_profile_states(&profiles);
            drop(profiles);
        }

        let _ = profiles_state.save(&self.fs).await;
    }
}

#[derive(Debug)]
pub struct ProfilesLoading<
    C: Io + DeleteAccountExt + Debug,
    F: Deref<
            Target = dyn Fn(
                PathBuf,
            )
                -> Pin<Box<dyn Future<Output = anyhow::Result<C>> + Sync + Send>>,
        > + Debug
        + Sync
        + Send,
> {
    pub profiles: parking_lot::Mutex<ActiveProfiles<C>>,
    pub factory: Arc<F>,
    pub secure_base_path: PathBuf,
    fs: Fs,
}

impl<
        C: Io + DeleteAccountExt + Debug,
        F: Deref<
                Target = dyn Fn(
                    PathBuf,
                )
                    -> Pin<Box<dyn Future<Output = anyhow::Result<C>> + Sync + Send>>,
            > + Debug
            + Sync
            + Send,
    > ProfilesLoading<C, F>
{
    pub async fn new(secure_base_path: PathBuf, factory: Arc<F>) -> anyhow::Result<Self> {
        let fs = Fs::new(secure_base_path.clone()).await?;
        let profiles_state = ProfilesState::load_or_default(&fs).await;
        let mut profiles: HashMap<String, ActiveProfile<C>> = Default::default();
        for (profile_key, profile) in profiles_state.profiles {
            profiles.insert(
                profile_key.clone(),
                ActiveProfile {
                    client: Arc::new(factory(secure_base_path.join(profile_key)).await?),
                    cur_cert: Default::default(),
                    profile_data: profile,
                },
            );
        }
        Ok(Self {
            profiles: parking_lot::Mutex::new(ActiveProfiles {
                profiles,
                cur_profile: profiles_state.cur_profile,
            }),
            factory,
            fs,
            secure_base_path,
        })
    }
}
