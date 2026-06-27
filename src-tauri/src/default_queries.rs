use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DefaultQuery {
    pub mode:       String,
    pub filter:     String,
    pub sort:       String,
    pub projection: String,
    pub skip:       i64,
    pub limit:      i64,
    pub pipeline:   String,
}

pub struct DefaultQueryStorage {
    path: PathBuf,
    lock: Mutex<()>,
}

impl DefaultQueryStorage {
    pub fn new(path: PathBuf) -> Self {
        Self { path: path, lock: Mutex::new(()) }
    }

    fn load_all(&self) -> HashMap<String, DefaultQuery> {
        if !self.path.exists() {
            return HashMap::new();
        }
        let content = std::fs::read_to_string(&self.path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    }

    fn save_all(&self, map: &HashMap<String, DefaultQuery>) -> Result<(), AppError> {
        let content = match serde_json::to_string_pretty(map) {
            Ok(val) => val,
            Err(e)  => return Err(AppError::Serde(e)),
        };
        crate::persist::atomic_write(&self.path, &content)
    }

    pub fn get(&self, key: &str) -> Option<DefaultQuery> {
        self.load_all().remove(key)
    }

    pub fn set(&self, key: &str, entry: DefaultQuery) -> Result<(), AppError> {
        let _guard = match self.lock.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let mut map = self.load_all();
        map.insert(key.to_string(), entry);
        self.save_all(&map)
    }

    pub fn clear(&self, key: &str) -> Result<(), AppError> {
        let _guard = match self.lock.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let mut map = self.load_all();
        map.remove(key);
        self.save_all(&map)
    }
}
