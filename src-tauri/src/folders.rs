use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

/// A connection-organizing folder shown in the Connection Manager grid. Folders
/// are their own entities (persisted in `folders.json`) so an empty folder can
/// exist before any connection is moved into it. `parent_id` is reserved for a
/// future nested-folder model; in the current flat model it is always `None`.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Folder {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    pub created_at: String,
}

pub struct FolderStorage {
    path: PathBuf,
    lock: Mutex<()>,
}

impl FolderStorage {
    pub fn new(path: PathBuf) -> Self {
        Self { path: path, lock: Mutex::new(()) }
    }

    pub fn load(&self) -> Vec<Folder> {
        if !self.path.exists() {
            return Vec::new();
        }
        let content = std::fs::read_to_string(&self.path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    }

    fn save(&self, folders: &[Folder]) -> Result<(), AppError> {
        let content = match serde_json::to_string_pretty(folders) {
            Ok(val) => val,
            Err(e) => return Err(AppError::Serde(e)),
        };
        crate::persist::atomic_write(&self.path, &content)
    }

    pub fn insert(&self, folder: Folder) -> Result<(), AppError> {
        let _guard = match self.lock.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let mut folders = self.load();
        folders.push(folder);
        self.save(&folders)
    }

    pub fn rename(&self, id: &str, name: &str) -> Result<(), AppError> {
        let _guard = match self.lock.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let mut folders = self.load();
        if let Some(f) = folders.iter_mut().find(|f| f.id == id) {
            f.name = name.to_string();
        }
        self.save(&folders)
    }

    pub fn delete(&self, id: &str) -> Result<(), AppError> {
        let _guard = match self.lock.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let mut folders = self.load();
        folders.retain(|f| f.id != id);
        self.save(&folders)
    }
}
