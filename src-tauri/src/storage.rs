use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

fn default_host() -> String { String::from("localhost") }
fn default_port() -> u16 { 27017 }
fn default_connection_type() -> String { String::from("standalone") }

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ConnectionConfig {
    pub id: String,
    pub name: String,
    #[serde(default = "default_host")]
    pub host: String,
    #[serde(default = "default_port")]
    pub port: u16,
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
    #[serde(default)]
    pub tag: Option<String>,
    #[serde(default)]
    pub last_accessed: Option<String>,
}

pub struct Storage {
    path: PathBuf,
}

impl Storage {
    pub fn new(path: PathBuf) -> Self {
        Self { path: path }
    }

    pub fn load(&self) -> Vec<ConnectionConfig> {
        if !self.path.exists() {
            return vec![];
        }
        let content = std::fs::read_to_string(&self.path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    }

    pub fn save(&self, connections: &[ConnectionConfig]) -> Result<(), AppError> {
        if let Some(parent) = self.path.parent() {
            match std::fs::create_dir_all(parent) {
                Ok(val) => val,
                Err(e) => return Err(AppError::Io(e)),
            };
        }
        let content = match serde_json::to_string_pretty(connections) {
            Ok(val) => val,
            Err(e) => return Err(AppError::Serde(e)),
        };
        match std::fs::write(&self.path, content) {
            Ok(val) => val,
            Err(e) => return Err(AppError::Io(e)),
        };
        Ok(())
    }

    pub fn add(&self, config: ConnectionConfig) -> Result<(), AppError> {
        let mut connections = self.load();
        connections.push(config);
        self.save(&connections)
    }

    pub fn update(&self, config: ConnectionConfig) -> Result<(), AppError> {
        let mut connections = self.load();
        if let Some(c) = connections.iter_mut().find(|c| c.id == config.id) {
            *c = config;
        }
        self.save(&connections)
    }

    pub fn remove(&self, id: &str) -> Result<(), AppError> {
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
            host: String::from("localhost"),
            port: 27017,
            connection_type: String::from("standalone"),
            replica_set_name: None,
            username: None,
            auth_db: None,
            auth_mechanism: None,
            tag: None,
            last_accessed: None,
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
    fn save_creates_parent_directories() {
        let dir = tempdir().unwrap();
        let nested_path = dir.path().join("a").join("b").join("connections.json");
        let storage = Storage::new(nested_path);
        storage.save(&[conn("1", "Local")]).unwrap();
        assert_eq!(storage.load().len(), 1);
    }
}
