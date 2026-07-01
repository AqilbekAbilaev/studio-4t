use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Mutex;

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

pub struct Storage {
    path: PathBuf,
    // Serializes read-modify-write sequences so concurrent commands can't lose
    // each other's updates to the same file.
    lock: Mutex<()>,
}

impl Storage {
    pub fn new(path: PathBuf) -> Self {
        Self { path: path, lock: Mutex::new(()) }
    }

    pub fn load(&self) -> Vec<ConnectionConfig> {
        if !self.path.exists() {
            return vec![];
        }
        let content = std::fs::read_to_string(&self.path).unwrap_or_default();
        let migrated = migrate_legacy_hosts(&content);
        serde_json::from_str(&migrated).unwrap_or_default()
    }

    pub fn save(&self, connections: &[ConnectionConfig]) -> Result<(), AppError> {
        let content = match serde_json::to_string_pretty(connections) {
            Ok(val) => val,
            Err(e) => return Err(AppError::Serde(e)),
        };
        crate::persist::atomic_write(&self.path, &content)
    }

    pub fn add(&self, config: ConnectionConfig) -> Result<(), AppError> {
        let _guard = match self.lock.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let mut connections = self.load();
        connections.push(config);
        self.save(&connections)
    }

    pub fn update(&self, config: ConnectionConfig) -> Result<(), AppError> {
        let _guard = match self.lock.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let mut connections = self.load();
        if let Some(c) = connections.iter_mut().find(|c| c.id == config.id) {
            *c = config;
        }
        self.save(&connections)
    }

    pub fn remove(&self, id: &str) -> Result<(), AppError> {
        let _guard = match self.lock.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let mut connections = self.load();
        connections.retain(|c| c.id != id);
        self.save(&connections)
    }

    /// The persisted config for `id`, if any. This is the authoritative source of
    /// a connection's URI — commands resolve it here rather than trusting the
    /// frontend to send the URI on every call.
    pub fn find(&self, id: &str) -> Option<ConnectionConfig> {
        self.load().into_iter().find(|c| c.id == id)
    }
}

#[cfg(test)]
mod tests;
