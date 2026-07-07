use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

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
    lock: Mutex<()>,
}

impl SavedQueryStorage {
    pub fn new(path: PathBuf) -> Self {
        Self { path: path, lock: Mutex::new(()) }
    }

    pub fn load(&self) -> Vec<SavedQueryEntry> {
        // Missing file -> empty; a present-but-corrupt file is quarantined aside
        // (not silently emptied) so the next save can't overwrite it. See
        // persist::read_json.
        crate::persist::read_json(&self.path).unwrap_or_else(Vec::new)
    }

    fn save(&self, entries: &[SavedQueryEntry]) -> Result<(), AppError> {
        let content = match serde_json::to_string_pretty(entries) {
            Ok(val) => val,
            Err(e) => return Err(AppError::Serde(e)),
        };
        crate::persist::atomic_write(&self.path, &content)
    }

    pub fn insert(&self, entry: SavedQueryEntry) -> Result<(), AppError> {
        let _guard = match self.lock.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let mut entries = self.load();
        entries.insert(0, entry);
        self.save(&entries)
    }

    pub fn delete(&self, id: &str) -> Result<(), AppError> {
        let _guard = match self.lock.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let mut entries = self.load();
        entries.retain(|e| e.id != id);
        self.save(&entries)
    }
}
