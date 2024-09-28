use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use crate::file_watcher::FileWatcher;

#[derive(Debug, Default)]
pub struct EmailDomainDenyList {
    pub domains: HashSet<url::Host>,
}

impl EmailDomainDenyList {
    pub fn is_banned(&self, email: &email_address::EmailAddress) -> bool {
        !url::Host::parse(&email.domain().to_lowercase())
            .is_ok_and(|host| !self.domains.contains(&host))
    }

    const PATH: &str = "config/";
    const FILE: &str = "email_domain_ban.txt";
    fn file_path() -> PathBuf {
        let path: &Path = Self::PATH.as_ref();
        path.join(Self::FILE)
    }
    pub async fn load_from_file() -> Self {
        let mut res = Self::default();
        match tokio::fs::read_to_string(Self::file_path()).await {
            Ok(file) => {
                for line in file.lines() {
                    match url::Host::parse(line) {
                        Ok(host) => {
                            res.domains.insert(host);
                        }
                        Err(err) => {
                            log::error!("{err}");
                        }
                    }
                }
            }
            Err(err) => {
                if matches!(err.kind(), std::io::ErrorKind::NotFound) {
                    let _ = tokio::fs::write(Self::file_path(), vec![]).await;
                } else {
                    log::error!("{err}");
                }
            }
        }
        res
    }

    pub fn watcher() -> FileWatcher {
        FileWatcher::new(Self::PATH.as_ref(), Self::FILE.as_ref())
    }
}

/// Checks if a email domain is allowed.
/// If the list is empty, all domains are allowed.
#[derive(Debug, Default)]
pub struct EmailDomainAllowList {
    pub domains: HashSet<url::Host>,
}

impl EmailDomainAllowList {
    pub fn is_allowed(&self, email: &email_address::EmailAddress) -> bool {
        self.domains.is_empty()
            || url::Host::parse(&email.domain().to_lowercase())
                .is_ok_and(|host| self.domains.contains(&host))
    }

    const PATH: &str = "config/";
    const FILE: &str = "email_domain_allow.txt";
    fn file_path() -> PathBuf {
        let path: &Path = Self::PATH.as_ref();
        path.join(Self::FILE)
    }

    pub async fn load_from_file() -> Self {
        let mut res = Self::default();
        match tokio::fs::read_to_string(Self::file_path()).await {
            Ok(file) => {
                for line in file.lines() {
                    match url::Host::parse(line) {
                        Ok(host) => {
                            res.domains.insert(host);
                        }
                        Err(err) => {
                            log::error!("{err}");
                        }
                    }
                }
            }
            Err(err) => {
                if matches!(err.kind(), std::io::ErrorKind::NotFound) {
                    let _ = tokio::fs::write(Self::file_path(), vec![]).await;
                } else {
                    log::error!("{err}");
                }
            }
        }
        res
    }

    pub fn watcher() -> FileWatcher {
        FileWatcher::new(Self::PATH.as_ref(), Self::FILE.as_ref())
    }
}
