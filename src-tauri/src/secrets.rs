//! Account credentials (Steam API key, GOG tokens, Epic session) and where
//! they are kept.
//!
//! In memory they stay on `Account`, so the sync code reads and writes them as
//! before. `AppStore::save` decides where they go on disk:
//!
//! - **release builds**: the macOS Keychain, one generic password per account
//!   (service `com.cartridge.app`, account = `Account::id`, value = JSON of
//!   `Secrets`). `library.json` holds none.
//! - **debug builds**: `library.json`, as before, so `pnpm tauri dev` doesn't
//!   ask for Keychain access after every rebuild (each build is a new unsigned
//!   binary). The Keychain is only read as a fallback, for accounts whose
//!   credentials a release build already moved there.

use serde::{Deserialize, Serialize};

use crate::models::Account;

pub const SERVICE: &str = "com.cartridge.app";

/// The credential fields of an `Account`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Secrets {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub access_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub egs_session: Option<String>,
}

impl Secrets {
    pub fn of(a: &Account) -> Self {
        Self {
            api_key: a.api_key.clone(),
            access_token: a.access_token.clone(),
            refresh_token: a.refresh_token.clone(),
            egs_session: a.egs_session.clone(),
        }
    }

    /// Clears the credential fields of `a`.
    pub fn strip(a: &mut Account) {
        a.api_key = None;
        a.access_token = None;
        a.refresh_token = None;
        a.egs_session = None;
    }

    /// Fills the credential fields of `a` that are empty.
    pub fn fill(&self, a: &mut Account) {
        fn set(slot: &mut Option<String>, v: &Option<String>) {
            if slot.is_none() {
                slot.clone_from(v);
            }
        }
        set(&mut a.api_key, &self.api_key);
        set(&mut a.access_token, &self.access_token);
        set(&mut a.refresh_token, &self.refresh_token);
        set(&mut a.egs_session, &self.egs_session);
    }

    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

/// Where credentials are stored outside `library.json`.
pub trait Vault: Send {
    /// `Ok(None)` when there is no entry for this account.
    fn get(&self, account_id: &str) -> anyhow::Result<Option<Secrets>>;
    fn set(&self, account_id: &str, secrets: &Secrets) -> anyhow::Result<()>;
    /// Removing a missing entry is not an error.
    fn delete(&self, account_id: &str) -> anyhow::Result<()>;
}

/// The macOS Keychain (through `keyring`).
pub struct Keychain;

impl Keychain {
    fn entry(account_id: &str) -> anyhow::Result<keyring::Entry> {
        Ok(keyring::Entry::new(SERVICE, account_id)?)
    }
}

impl Vault for Keychain {
    fn get(&self, account_id: &str) -> anyhow::Result<Option<Secrets>> {
        match Self::entry(account_id)?.get_password() {
            Ok(json) => Ok(Some(serde_json::from_str(&json)?)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn set(&self, account_id: &str, secrets: &Secrets) -> anyhow::Result<()> {
        Self::entry(account_id)?.set_password(&serde_json::to_string(secrets)?)?;
        Ok(())
    }

    fn delete(&self, account_id: &str) -> anyhow::Result<()> {
        match Self::entry(account_id)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}

#[cfg(test)]
pub mod testing {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    /// In-memory vault; clones share their contents, so a test keeps one to
    /// inspect what the store wrote.
    #[derive(Clone, Default)]
    pub struct FakeVault {
        pub entries: Arc<Mutex<HashMap<String, Secrets>>>,
        /// Every call fails while set (a denied Keychain prompt, say).
        pub failing: Arc<Mutex<bool>>,
    }

    impl FakeVault {
        fn check(&self) -> anyhow::Result<()> {
            if *self.failing.lock().unwrap() {
                anyhow::bail!("vault unavailable");
            }
            Ok(())
        }
        pub fn fail(&self, on: bool) {
            *self.failing.lock().unwrap() = on;
        }
        pub fn get_now(&self, id: &str) -> Option<Secrets> {
            self.entries.lock().unwrap().get(id).cloned()
        }
    }

    impl Vault for FakeVault {
        fn get(&self, id: &str) -> anyhow::Result<Option<Secrets>> {
            self.check()?;
            Ok(self.get_now(id))
        }
        fn set(&self, id: &str, s: &Secrets) -> anyhow::Result<()> {
            self.check()?;
            self.entries.lock().unwrap().insert(id.into(), s.clone());
            Ok(())
        }
        fn delete(&self, id: &str) -> anyhow::Result<()> {
            self.check()?;
            self.entries.lock().unwrap().remove(id);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Against the real Keychain: `cargo test -- --ignored live_keychain`.
    #[test]
    #[ignore]
    fn live_keychain_roundtrip() {
        let id = format!("cartridge-selftest-{}", std::process::id());
        let s = Secrets {
            api_key: Some("test-key".into()),
            ..Default::default()
        };
        Keychain.set(&id, &s).unwrap();
        assert_eq!(Keychain.get(&id).unwrap(), Some(s));
        Keychain.delete(&id).unwrap();
        assert_eq!(Keychain.get(&id).unwrap(), None);
        Keychain.delete(&id).unwrap(); // missing entry: fine
    }

    #[test]
    fn fill_keeps_what_the_account_has() {
        let mut a: Account = serde_json::from_value(serde_json::json!({
            "id": "1", "name": "GOG", "handle": "h", "platform": "gog", "count": 0,
            "synced": "", "status": "ok", "accessToken": "fresh",
        }))
        .unwrap();
        let stored = Secrets {
            access_token: Some("old".into()),
            refresh_token: Some("refresh".into()),
            ..Default::default()
        };
        stored.fill(&mut a);
        assert_eq!(a.access_token.as_deref(), Some("fresh"));
        assert_eq!(a.refresh_token.as_deref(), Some("refresh"));
        Secrets::strip(&mut a);
        assert!(Secrets::of(&a).is_empty());
    }
}
