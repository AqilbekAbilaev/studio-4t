use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Mutex, MutexGuard};

fn default_connection_type() -> String { String::from("standalone") }
fn default_ssh_port() -> u16 { 22 }

/// One host of a (possibly multi-host) seed list. SRV connections use a single
/// entry and ignore the port.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct HostEntry {
    pub host: String,
    pub port: u16,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ConnectionConfig {
    pub id: String,
    pub name: String,
    /// The seed list. Always holds at least one entry after `load()` migrates
    /// legacy single-host configs (see `migrate_legacy_hosts`).
    #[serde(default)]
    pub hosts: Vec<HostEntry>,
    #[serde(default = "default_connection_type")]
    pub connection_type: String,
    #[serde(default)]
    pub replica_set_name: Option<String>,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub auth_db: Option<String>,
    #[serde(default)]
    pub auth_mechanism: Option<String>,
    // Passthrough connection-string options the dedicated fields don't model
    // (e.g. retryWrites, socketTimeoutMS, readPreference). Appended verbatim to
    // the built URI so any driver-accepted parameter round-trips.
    #[serde(default)]
    pub options: BTreeMap<String, String>,
    // TLS / SSL. `tls` enables it; the file paths and allow-invalid flag are
    // applied as connection-string options (see uri::build_uri).
    #[serde(default)]
    pub tls: bool,
    #[serde(default)]
    pub tls_ca_file: Option<String>,
    #[serde(default)]
    pub tls_cert_key_file: Option<String>,
    #[serde(default)]
    pub tls_allow_invalid_certificates: bool,
    // SSH tunnel. When enabled, the driver connects through a local forwarded
    // port (see ssh.rs / pool::connect). Secrets live in the keychain, not here.
    #[serde(default)]
    pub ssh_enabled: bool,
    #[serde(default)]
    pub ssh_host: Option<String>,
    #[serde(default = "default_ssh_port")]
    pub ssh_port: u16,
    #[serde(default)]
    pub ssh_user: Option<String>,
    #[serde(default)]
    pub ssh_auth: Option<String>,
    #[serde(default)]
    pub ssh_key_file: Option<String>,
    #[serde(default)]
    pub tag: Option<String>,
    // The folder this connection belongs to in the Connection Manager, or `None`
    // for a connection at the root. Folders themselves live in `folders.json`.
    #[serde(default)]
    pub folder_id: Option<String>,
    #[serde(default)]
    pub last_accessed: Option<String>,
    // Whether the connection is currently open in the sidebar tree. Persisted so
    // only the connections that were open are re-opened after a restart.
    #[serde(default)]
    pub open: bool,
}

/// Rewrites legacy pre-multi-host connection JSON so it parses into the current
/// `ConnectionConfig`: a top-level `host`/`port` pair becomes a one-element
/// `hosts` array, and the old `"dns"` SRV type tag becomes `"srv"`. Returns the
/// input unchanged if it isn't the expected array-of-objects shape.
fn migrate_legacy_hosts(content: &str) -> String {
    let mut value: serde_json::Value = match serde_json::from_str(content) {
        Ok(val) => val,
        Err(_) => return content.to_string(),
    };
    let array = match value.as_array_mut() {
        Some(arr) => arr,
        None => return content.to_string(),
    };
    for item in array.iter_mut() {
        let obj = match item.as_object_mut() {
            Some(o) => o,
            None => continue,
        };
        if obj.get("connection_type").and_then(|v| v.as_str()) == Some("dns") {
            obj.insert(
                String::from("connection_type"),
                serde_json::Value::String(String::from("srv")),
            );
        }
        if !obj.contains_key("hosts") {
            let host = obj
                .get("host")
                .and_then(|v| v.as_str())
                .unwrap_or("localhost")
                .to_string();
            let port = obj.get("port").and_then(|v| v.as_u64()).unwrap_or(27017);
            let entry = serde_json::json!({ "host": host, "port": port });
            obj.insert(
                String::from("hosts"),
                serde_json::Value::Array(vec![entry]),
            );
        }
    }
    match serde_json::to_string(&value) {
        Ok(val) => val,
        Err(_) => content.to_string(),
    }
}

// Intentionally bespoke — not a JsonStore<T>: this store owns an in-memory cache
// (the source of truth once loaded) and runs `migrate_legacy_hosts` on read, so
// its read/write path diverges from the plain load-mutate-save the generic covers.
pub struct Storage {
    path: PathBuf,
    // Cached connection list — the in-memory source of truth once loaded. `None`
    // until first access (lazy load) or after a failed write. Every read is served
    // from here; every mutation updates this and the file together under the lock,
    // so a read never hits disk on a cache hit. The lock also serializes
    // read-modify-write sequences so concurrent commands can't lose each other's
    // updates.
    //
    // Tradeoff: external hand-edits to connections.json while the app is running
    // are not observed until the next mutation or an app restart. Live external
    // edits are unsupported, so this is acceptable.
    cache: Mutex<Option<Vec<ConnectionConfig>>>,
}

impl Storage {
    pub fn new(path: PathBuf) -> Self {
        Self { path: path, cache: Mutex::new(None) }
    }

    // The poison-tolerant cache guard. A panic in another thread while the lock is
    // held poisons it; we recover the inner data rather than propagate the panic.
    fn lock_cache(&self) -> MutexGuard<'_, Option<Vec<ConnectionConfig>>> {
        match self.cache.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    // Reads, migrates, and parses the list straight from disk. Does not touch the
    // lock, so it can run inside a held `lock_cache()` guard without re-entrancy.
    // Called only on a cache miss / first load.
    fn read_from_disk(&self) -> Vec<ConnectionConfig> {
        if !self.path.exists() {
            return vec![];
        }
        // A file that exists but can't be read/parsed is quarantined aside (not
        // silently emptied), so the next write persists a fresh file instead of
        // overwriting the recoverable original. See persist::quarantine_corrupt.
        let content = match std::fs::read_to_string(&self.path) {
            Ok(value) => value,
            Err(error) => {
                eprintln!(
                    "storage: failed to read {}: {}",
                    self.path.display(),
                    error
                );
                crate::persist::quarantine_corrupt(&self.path);
                return vec![];
            }
        };
        let migrated = migrate_legacy_hosts(&content);
        match serde_json::from_str(&migrated) {
            Ok(value) => value,
            Err(error) => {
                eprintln!(
                    "storage: failed to parse {}: {}",
                    self.path.display(),
                    error
                );
                crate::persist::quarantine_corrupt(&self.path);
                vec![]
            }
        }
    }

    // Serializes and atomically writes the list. Pure disk write, no lock.
    fn write_disk(&self, connections: &[ConnectionConfig]) -> Result<(), AppError> {
        let content = match serde_json::to_string_pretty(connections) {
            Ok(val) => val,
            Err(e) => return Err(AppError::Serde(e)),
        };
        crate::persist::atomic_write(&self.path, &content)
    }

    pub fn load(&self) -> Vec<ConnectionConfig> {
        let mut guard = self.lock_cache();
        let connections = guard.get_or_insert_with(|| self.read_from_disk());
        connections.clone()
    }

    /// The persisted config for `id`, if any. This is the authoritative source of
    /// a connection's URI — commands resolve it here rather than trusting the
    /// frontend to send the URI on every call.
    pub fn find(&self, id: &str) -> Option<ConnectionConfig> {
        let mut guard = self.lock_cache();
        let connections = guard.get_or_insert_with(|| self.read_from_disk());
        connections.iter().find(|c| c.id == id).cloned()
    }

    /// Apply `mutate` to the connection list under the lock, persist, then sync the
    /// cache. The one cache-consistent write core: `add`/`update`/`remove` delegate
    /// here so no mutation path can forget to update the cache, and two concurrent
    /// field updates (e.g. set-open + set-last-accessed) still serialize on the one
    /// lock so neither is lost.
    pub fn update_with<F>(&self, mutate: F) -> Result<(), AppError>
    where
        F: FnOnce(&mut Vec<ConnectionConfig>),
    {
        let mut guard = self.lock_cache();
        let mut connections = match guard.take() {
            Some(connections) => connections,
            None => self.read_from_disk(),
        };
        mutate(&mut connections);
        match self.write_disk(&connections) {
            Ok(()) => {
                *guard = Some(connections);
                Ok(())
            }
            // Persist failed: don't cache the unpersisted change. Leaving the cache
            // empty forces the next read to reload the last good state from disk.
            Err(e) => {
                *guard = None;
                Err(e)
            }
        }
    }

    pub fn add(&self, config: ConnectionConfig) -> Result<(), AppError> {
        self.update_with(|connections| connections.push(config))
    }

    pub fn update(&self, config: ConnectionConfig) -> Result<(), AppError> {
        self.update_with(|connections| {
            if let Some(c) = connections.iter_mut().find(|c| c.id == config.id) {
                *c = config;
            }
        })
    }

    pub fn remove(&self, id: &str) -> Result<(), AppError> {
        self.update_with(|connections| connections.retain(|c| c.id != id))
    }

    // Replace the whole list: write to disk, then sync the cache. Only the tests
    // use this — the app mutates through `update_with` and its `add`/`update`/
    // `remove` delegates, so it's compiled in test builds only.
    #[cfg(test)]
    fn save(&self, connections: &[ConnectionConfig]) -> Result<(), AppError> {
        let mut guard = self.lock_cache();
        match self.write_disk(connections) {
            Ok(()) => {
                *guard = Some(connections.to_vec());
                Ok(())
            }
            Err(e) => {
                *guard = None;
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests;
