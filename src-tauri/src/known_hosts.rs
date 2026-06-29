// Trust-on-first-use (TOFU) store for SSH host keys.
//
// The first time we connect to a bastion we record its public key here; on every
// later connection we compare the presented key against the stored one. A match
// is accepted, a brand-new host is recorded and accepted, and a *changed* key is
// rejected (possible man-in-the-middle, or a rotated server key). This replaces
// the previous accept-all policy in ssh.rs.
//
// Keyed by `host:port`, not by connection id, because several connections can
// share one bastion and should trust the same key.

use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::UNIX_EPOCH;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct KnownHost {
    pub host: String,
    pub port: u16,
    // The server's public key in OpenSSH authorized_keys form (`<algo> <base64>`).
    pub key: String,
    // Milliseconds since the Unix epoch when this key was first trusted.
    pub added: String,
}

/// The result of comparing a presented key against what we have on file.
#[derive(Debug, PartialEq)]
pub enum HostKeyCheck {
    /// The presented key matches the stored one.
    Match,
    /// No key on file for this host yet (first contact).
    Unknown,
    /// A different key is already stored for this host.
    Changed,
}

/// Pure three-way decision, separated from any I/O so it can be unit-tested.
pub fn classify(stored: Option<&str>, presented: &str) -> HostKeyCheck {
    match stored {
        None => HostKeyCheck::Unknown,
        Some(value) => {
            if value == presented {
                HostKeyCheck::Match
            } else {
                HostKeyCheck::Changed
            }
        }
    }
}

pub struct KnownHostsStore {
    path: PathBuf,
    // Serializes the load-modify-save sequence so two first-contact connections
    // can't lose each other's recorded keys.
    lock: Mutex<()>,
}

impl KnownHostsStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path: path, lock: Mutex::new(()) }
    }

    fn load_all(&self) -> Vec<KnownHost> {
        if !self.path.exists() {
            return Vec::new();
        }
        let content = std::fs::read_to_string(&self.path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    }

    fn save_all(&self, hosts: &[KnownHost]) -> Result<(), AppError> {
        let content = match serde_json::to_string_pretty(hosts) {
            Ok(value) => value,
            Err(e) => return Err(AppError::Serde(e)),
        };
        crate::persist::atomic_write(&self.path, &content)
    }

    /// TOFU check. Returns `Ok(())` when the key matches a stored entry or is
    /// recorded as a new first-contact host. Returns `Err` when a *different*
    /// key is already on file for this host — the caller must refuse to connect.
    pub fn verify_or_record(&self, host: &str, port: u16, key: &str) -> Result<(), AppError> {
        let _guard = match self.lock.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let mut hosts = self.load_all();
        let stored = hosts
            .iter()
            .find(|h| h.host == host && h.port == port)
            .map(|h| h.key.as_str());
        match classify(stored, key) {
            HostKeyCheck::Match => Ok(()),
            HostKeyCheck::Unknown => {
                hosts.push(KnownHost {
                    host: host.to_string(),
                    port: port,
                    key: key.to_string(),
                    added: now_ms(),
                });
                self.save_all(&hosts)
            }
            HostKeyCheck::Changed => Err(AppError::Ssh(format!(
                "host key verification failed for {}:{} — the server's key does not match the \
                 previously trusted key. This may indicate a man-in-the-middle attack, or the \
                 server's key was rotated. Connection refused.",
                host, port
            ))),
        }
    }
}

fn now_ms() -> String {
    let ms = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("{}", ms)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn store_in_tempdir() -> (KnownHostsStore, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let store = KnownHostsStore::new(dir.path().join("known_hosts.json"));
        (store, dir)
    }

    #[test]
    fn classify_unknown_when_nothing_stored() {
        assert_eq!(classify(None, "ssh-ed25519 AAAA"), HostKeyCheck::Unknown);
    }

    #[test]
    fn classify_match_when_equal() {
        assert_eq!(
            classify(Some("ssh-ed25519 AAAA"), "ssh-ed25519 AAAA"),
            HostKeyCheck::Match
        );
    }

    #[test]
    fn classify_changed_when_different() {
        assert_eq!(
            classify(Some("ssh-ed25519 AAAA"), "ssh-ed25519 BBBB"),
            HostKeyCheck::Changed
        );
    }

    #[test]
    fn first_contact_is_recorded_and_accepted() {
        let (store, _dir) = store_in_tempdir();
        let result = store.verify_or_record("bastion.example.com", 22, "ssh-ed25519 AAAA");
        assert!(result.is_ok());
        let hosts = store.load_all();
        assert_eq!(hosts.len(), 1);
        assert_eq!(hosts[0].host, "bastion.example.com");
        assert_eq!(hosts[0].key, "ssh-ed25519 AAAA");
    }

    #[test]
    fn same_key_on_reconnect_is_accepted() {
        let (store, _dir) = store_in_tempdir();
        store
            .verify_or_record("bastion.example.com", 22, "ssh-ed25519 AAAA")
            .unwrap();
        let result = store.verify_or_record("bastion.example.com", 22, "ssh-ed25519 AAAA");
        assert!(result.is_ok());
        // No duplicate entry recorded.
        assert_eq!(store.load_all().len(), 1);
    }

    #[test]
    fn changed_key_is_rejected() {
        let (store, _dir) = store_in_tempdir();
        store
            .verify_or_record("bastion.example.com", 22, "ssh-ed25519 AAAA")
            .unwrap();
        let result = store.verify_or_record("bastion.example.com", 22, "ssh-ed25519 BBBB");
        assert!(result.is_err());
        // The trusted key is left untouched.
        assert_eq!(store.load_all()[0].key, "ssh-ed25519 AAAA");
    }

    #[test]
    fn different_port_is_a_separate_host() {
        let (store, _dir) = store_in_tempdir();
        store
            .verify_or_record("bastion.example.com", 22, "ssh-ed25519 AAAA")
            .unwrap();
        // Same host name, different port → unknown, recorded independently.
        let result = store.verify_or_record("bastion.example.com", 2222, "ssh-ed25519 BBBB");
        assert!(result.is_ok());
        assert_eq!(store.load_all().len(), 2);
    }
}
