use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

use crate::session::paths;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum AuthEntry {
    Api {
        key: String,
    },
    Oauth {
        refresh:     String,
        access:      String,
        expires:     u64,
        #[serde(rename = "accountId", skip_serializing_if = "Option::is_none")]
        account_id:  Option<String>,
    },
}

impl AuthEntry {
    pub fn api_key(&self) -> Option<&str> {
        match self {
            Self::Api { key } => Some(key.as_str()),
            Self::Oauth { access, .. } => Some(access.as_str()),
        }
    }

    pub fn is_expired(&self) -> bool {
        match self {
            Self::Api { .. } => false,
            Self::Oauth { expires, .. } => {
                let now_ms = chrono::Utc::now().timestamp_millis() as u64;
                *expires > 0 && now_ms >= *expires
            }
        }
    }

    pub fn refresh_token(&self) -> Option<&str> {
        match self {
            Self::Api { .. } => None,
            Self::Oauth { refresh, .. } => Some(refresh.as_str()),
        }
    }
}

pub type AuthStore = HashMap<String, AuthEntry>;

pub fn auth_path() -> PathBuf {
    paths::config_dir().join("auth.json")
}

pub fn load_auth_store() -> Option<AuthStore> {
    let path = auth_path();
    let raw = std::fs::read_to_string(&path).ok()?;
    match serde_json::from_str::<AuthStore>(&raw) {
        Ok(store) => {
            debug!(path = %path.display(), entries = store.len(), "auth store loaded");
            Some(store)
        }
        Err(e) => {
            warn!(path = %path.display(), error = %e, "failed to parse auth.json");
            None
        }
    }
}

pub fn save_auth_store(store: &AuthStore) -> std::io::Result<()> {
    let path = auth_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_vec_pretty(store)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&path, &json)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&path)?.permissions();
        perms.set_mode(0o600);
        std::fs::set_permissions(&path, perms)?;
    }

    info!(path = %path.display(), entries = store.len(), "auth store saved");
    Ok(())
}

pub fn remove_auth_entry(store: &mut AuthStore, provider_id: &str) -> bool {
    store.remove(provider_id).is_some()
}

pub fn get_api_key<'a>(store: &'a AuthStore, provider_id: &str) -> Option<&'a str> {
    store.get(provider_id)
        .filter(|e| !e.is_expired())
        .and_then(|e| e.api_key())
        .filter(|k| !k.trim().is_empty())
}
