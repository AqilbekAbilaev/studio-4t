use crate::error::AppError;
use crate::json_store::JsonStore;
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
    inner: JsonStore<Vec<SavedQueryEntry>>,
}

impl SavedQueryStorage {
    pub fn new(path: PathBuf) -> Self {
        Self { inner: JsonStore::new(path) }
    }

    pub fn load(&self) -> Vec<SavedQueryEntry> {
        self.inner.load()
    }

    pub fn insert(&self, entry: SavedQueryEntry) -> Result<(), AppError> {
        self.inner.update(|entries| entries.insert(0, entry))
    }

    pub fn delete(&self, id: &str) -> Result<(), AppError> {
        self.inner.update(|entries| entries.retain(|e| e.id != id))
    }
}
