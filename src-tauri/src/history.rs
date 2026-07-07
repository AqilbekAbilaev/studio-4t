use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::UNIX_EPOCH;

const MAX_HISTORY: usize = 50;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct QueryHistoryEntry {
    pub id: String,
    pub mode: String,
    pub filter: String,
    pub sort: String,
    pub projection: String,
    pub skip: i64,
    pub limit: i64,
    pub pipeline: String,
    pub ran_at: String,
}

pub struct HistoryStorage {
    path: PathBuf,
    lock: Mutex<()>,
}

impl HistoryStorage {
    pub fn new(path: PathBuf) -> Self {
        Self { path: path, lock: Mutex::new(()) }
    }

    fn load_all(&self) -> HashMap<String, Vec<QueryHistoryEntry>> {
        // Missing file -> empty; a present-but-corrupt file is quarantined aside
        // (not silently emptied) so the next save can't overwrite it. See
        // persist::read_json.
        crate::persist::read_json(&self.path).unwrap_or_else(HashMap::new)
    }

    fn save_all(&self, map: &HashMap<String, Vec<QueryHistoryEntry>>) -> Result<(), AppError> {
        let content = match serde_json::to_string_pretty(map) {
            Ok(val) => val,
            Err(e) => return Err(AppError::Serde(e)),
        };
        crate::persist::atomic_write(&self.path, &content)
    }

    pub fn get(&self, key: &str) -> Vec<QueryHistoryEntry> {
        let map = self.load_all();
        match map.get(key) {
            Some(entries) => entries.clone(),
            None => Vec::new(),
        }
    }

    pub fn push(&self, key: &str, entry: QueryHistoryEntry) -> Result<(), AppError> {
        let _guard = match self.lock.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let mut map = self.load_all();
        let entries = map.entry(key.to_string()).or_insert_with(Vec::new);
        entries.retain(|e| !is_same_query(e, &entry));
        entries.insert(0, entry);
        entries.truncate(MAX_HISTORY);
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

fn is_same_query(a: &QueryHistoryEntry, b: &QueryHistoryEntry) -> bool {
    a.mode == b.mode
        && a.filter == b.filter
        && a.sort == b.sort
        && a.projection == b.projection
        && a.skip == b.skip
        && a.limit == b.limit
        && a.pipeline == b.pipeline
}

pub fn now_ms() -> String {
    let ms = std::time::SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("{}", ms)
}
