use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

use axum::{
    body::Body,
    extract::{ConnectInfo, State},
    http::Request,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use ddnet_accounts_shared::account_server::{
    errors::AccountServerRequestError, result::AccountServerReqResult,
};
use parking_lot::RwLock;
use reqwest::StatusCode;

use crate::file_watcher::FileWatcher;

#[derive(Debug, Default)]
pub struct IpDenyList {
    pub ipv4: iprange::IpRange<ipnet::Ipv4Net>,
    pub ipv6: iprange::IpRange<ipnet::Ipv6Net>,
}

impl IpDenyList {
    pub fn is_banned(&self, addr: SocketAddr) -> bool {
        match addr {
            SocketAddr::V4(ip) => self.ipv4.contains(&ipnet::Ipv4Net::from(*ip.ip())),
            SocketAddr::V6(ip) => self.ipv6.contains(&ipnet::Ipv6Net::from(*ip.ip())),
        }
    }

    const PATH: &str = "config/";
    const FILE: &str = "ip_ban.txt";
    fn file_path() -> PathBuf {
        let path: &Path = Self::PATH.as_ref();
        path.join(Self::FILE)
    }

    pub async fn load_from_file() -> Self {
        let mut res = Self::default();
        match tokio::fs::read_to_string(Self::file_path()).await {
            Ok(file) => {
                for line in file.lines() {
                    match ipnet::IpNet::from_str(line) {
                        Ok(ip) => match ip {
                            ipnet::IpNet::V4(ipv4_net) => {
                                res.ipv4.add(ipv4_net);
                            }
                            ipnet::IpNet::V6(ipv6_net) => {
                                res.ipv6.add(ipv6_net);
                            }
                        },
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

pub async fn ip_deny_layer(
    State(deny_list): State<Arc<RwLock<IpDenyList>>>,
    ConnectInfo(client_ip): ConnectInfo<SocketAddr>,
    req: Request<Body>,
    next: Next,
) -> Result<Response<Body>, StatusCode> {
    if deny_list.read().is_banned(client_ip) {
        Ok(Json(AccountServerReqResult::<(), ()>::Err(
            AccountServerRequestError::VpnBan(
                "VPN detected. Please deactivate the VPN and try again.".to_string(),
            ),
        ))
        .into_response())
    } else {
        Ok(next.run(req).await)
    }
}
