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
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn conn(id: &str, name: &str) -> ConnectionConfig {
        ConnectionConfig {
            id: id.into(),
            name: name.into(),
            hosts: vec![HostEntry { host: String::from("localhost"), port: 27017 }],
            connection_type: String::from("standalone"),
            replica_set_name: None,
            username: None,
            auth_db: None,
            auth_mechanism: None,
            options: BTreeMap::new(),
            tls: false,
            tls_ca_file: None,
            tls_cert_key_file: None,
            tls_allow_invalid_certificates: false,
            ssh_enabled: false,
            ssh_host: None,
            ssh_port: 22,
            ssh_user: None,
            ssh_auth: None,
            ssh_key_file: None,
            tag: None,
            last_accessed: None,
            open: false,
        }
    }

    fn storage_in_tempdir() -> (Storage, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let s = Storage::new(dir.path().join("connections.json"));
        (s, dir)
    }

    #[test]
    fn load_returns_empty_when_file_missing() {
        let (storage, _dir) = storage_in_tempdir();
        assert!(storage.load().is_empty());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let (storage, _dir) = storage_in_tempdir();
        let conns = vec![
            conn("1", "Local"),
            conn("2", "Prod"),
        ];
        storage.save(&conns).unwrap();
        assert_eq!(storage.load(), conns);
    }

    #[test]
    fn save_overwrites_existing_file() {
        let (storage, _dir) = storage_in_tempdir();
        storage.save(&[conn("1", "Old")]).unwrap();
        storage.save(&[conn("1", "New")]).unwrap();
        let loaded = storage.load();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].name, "New");
    }

    #[test]
    fn load_returns_empty_on_corrupt_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("connections.json");
        std::fs::write(&path, "not valid json {{ garbage").unwrap();
        let storage = Storage::new(path);
        assert!(storage.load().is_empty());
    }

    #[test]
    fn add_appends_to_existing() {
        let (storage, _dir) = storage_in_tempdir();
        storage.add(conn("1", "Local")).unwrap();
        storage.add(conn("2", "Prod")).unwrap();
        let loaded = storage.load();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[1].id, "2");
    }

    #[test]
    fn remove_deletes_by_id() {
        let (storage, _dir) = storage_in_tempdir();
        storage.save(&[
            conn("1", "Keep"),
            conn("2", "Delete"),
        ]).unwrap();
        storage.remove("2").unwrap();
        let loaded = storage.load();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "1");
    }

    #[test]
    fn remove_nonexistent_id_is_noop() {
        let (storage, _dir) = storage_in_tempdir();
        storage.save(&[conn("1", "Local")]).unwrap();
        storage.remove("999").unwrap();
        assert_eq!(storage.load().len(), 1);
    }

    #[test]
    fn load_migrates_legacy_single_host() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("connections.json");
        // A pre-multi-host record: top-level host/port, no `hosts` array.
        std::fs::write(
            &path,
            r#"[{"id":"1","name":"Old","host":"db.example.com","port":27018,"connection_type":"standalone"}]"#,
        ).unwrap();
        let storage = Storage::new(path);
        let loaded = storage.load();
        assert_eq!(loaded.len(), 1);
        assert_eq!(
            loaded[0].hosts,
            vec![HostEntry { host: String::from("db.example.com"), port: 27018 }]
        );
    }

    #[test]
    fn load_migrates_legacy_dns_type_to_srv() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("connections.json");
        std::fs::write(
            &path,
            r#"[{"id":"1","name":"Atlas","host":"cluster.example.com","port":27017,"connection_type":"dns"}]"#,
        ).unwrap();
        let storage = Storage::new(path);
        let loaded = storage.load();
        assert_eq!(loaded[0].connection_type, "srv");
    }

    #[test]
    fn load_preserves_existing_hosts_array() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("connections.json");
        std::fs::write(
            &path,
            r#"[{"id":"1","name":"RS","hosts":[{"host":"a","port":1},{"host":"b","port":2}],"connection_type":"replica"}]"#,
        ).unwrap();
        let storage = Storage::new(path);
        let loaded = storage.load();
        assert_eq!(loaded[0].hosts.len(), 2);
        assert_eq!(loaded[0].hosts[1].host, "b");
    }

    #[test]
    fn save_creates_parent_directories() {
        let dir = tempdir().unwrap();
        let nested_path = dir.path().join("a").join("b").join("connections.json");
        let storage = Storage::new(nested_path);
        storage.save(&[conn("1", "Local")]).unwrap();
        assert_eq!(storage.load().len(), 1);
    }
}
