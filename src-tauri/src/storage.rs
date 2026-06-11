use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ConnectionConfig {
    pub id: String,
    pub name: String,
    pub uri: String,
    #[serde(default)]
    pub last_accessed: Option<String>,
    #[serde(default)]
    pub tag: Option<String>,
}

pub struct Storage {
    path: PathBuf,
}

impl Storage {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
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
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(connections)?;
        std::fs::write(&self.path, content)?;
        Ok(())
    }

    pub fn add(&self, config: ConnectionConfig) -> Result<(), AppError> {
        let mut connections = self.load();
        connections.push(config);
        self.save(&connections)
    }

    pub fn remove(&self, id: &str) -> Result<(), AppError> {
        let mut connections = self.load();
        connections.retain(|c| c.id != id);
        self.save(&connections)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn conn(id: &str, name: &str, uri: &str) -> ConnectionConfig {
        ConnectionConfig {
            id: id.into(),
            name: name.into(),
            uri: uri.into(),
            last_accessed: None,
            tag: None,
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
            conn("1", "Local", "mongodb://localhost:27017"),
            conn("2", "Prod", "mongodb://prod.example.com:27017"),
        ];
        storage.save(&conns).unwrap();
        assert_eq!(storage.load(), conns);
    }

    #[test]
    fn save_overwrites_existing_file() {
        let (storage, _dir) = storage_in_tempdir();
        storage.save(&[conn("1", "Old", "mongodb://old:27017")]).unwrap();
        storage.save(&[conn("1", "New", "mongodb://new:27017")]).unwrap();
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
        storage.add(conn("1", "Local", "mongodb://localhost:27017")).unwrap();
        storage.add(conn("2", "Prod", "mongodb://prod:27017")).unwrap();
        let loaded = storage.load();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[1].id, "2");
    }

    #[test]
    fn remove_deletes_by_id() {
        let (storage, _dir) = storage_in_tempdir();
        storage.save(&[
            conn("1", "Keep", "mongodb://keep:27017"),
            conn("2", "Delete", "mongodb://delete:27017"),
        ]).unwrap();
        storage.remove("2").unwrap();
        let loaded = storage.load();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].id, "1");
    }

    #[test]
    fn remove_nonexistent_id_is_noop() {
        let (storage, _dir) = storage_in_tempdir();
        storage.save(&[conn("1", "Local", "mongodb://localhost:27017")]).unwrap();
        storage.remove("999").unwrap();
        assert_eq!(storage.load().len(), 1);
    }

    #[test]
    fn save_creates_parent_directories() {
        let dir = tempdir().unwrap();
        let nested_path = dir.path().join("a").join("b").join("connections.json");
        let storage = Storage::new(nested_path);
        storage.save(&[conn("1", "Local", "mongodb://localhost:27017")]).unwrap();
        assert_eq!(storage.load().len(), 1);
    }
}
