use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SavedQueryEntry {
    pub id:         String,
    pub name:       String,
    pub mode:       String,
    pub filter:     String,
    pub sort:       String,
    pub projection: String,
    pub skip:       i64,
    pub limit:      i64,
    pub pipeline:   String,
    pub saved_at:   String,
}

pub struct SavedQueryStorage {
    path: PathBuf,
}

impl SavedQueryStorage {
    pub fn new(path: PathBuf) -> Self {
        Self { path: path }
    }

    pub fn load(&self) -> Vec<SavedQueryEntry> {
        if !self.path.exists() {
            return Vec::new();
        }
        let content = std::fs::read_to_string(&self.path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    }

    fn save(&self, entries: &[SavedQueryEntry]) -> Result<(), AppError> {
        if let Some(parent) = self.path.parent() {
            match std::fs::create_dir_all(parent) {
                Ok(val) => val,
                Err(e) => return Err(AppError::Io(e)),
            };
        }
        let content = match serde_json::to_string_pretty(entries) {
            Ok(val) => val,
            Err(e) => return Err(AppError::Serde(e)),
        };
        match std::fs::write(&self.path, content) {
            Ok(val) => val,
            Err(e) => return Err(AppError::Io(e)),
        };
        Ok(())
    }

    pub fn insert(&self, entry: SavedQueryEntry) -> Result<(), AppError> {
        let mut entries = self.load();
        entries.insert(0, entry);
        self.save(&entries)
    }

    pub fn delete(&self, id: &str) -> Result<(), AppError> {
        let mut entries = self.load();
        entries.retain(|e| e.id != id);
        self.save(&entries)
    }
}
