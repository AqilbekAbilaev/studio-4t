use crate::error::AppError;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

const MAX_HISTORY: usize = 200;

/// Per-connection IntelliShell command history, persisted to shell_history.json.
/// Commands are stored oldest-first so the frontend can use the list directly
/// for ↑/↓ recall.
pub struct ShellHistoryStorage {
    path: PathBuf,
    lock: Mutex<()>,
}

impl ShellHistoryStorage {
    pub fn new(path: PathBuf) -> Self {
        Self { path: path, lock: Mutex::new(()) }
    }

    fn load_all(&self) -> HashMap<String, Vec<String>> {
        // Missing file -> empty; a present-but-corrupt file is quarantined aside
        // (not silently emptied) so the next save can't overwrite it. See
        // persist::read_json.
        crate::persist::read_json(&self.path).unwrap_or_else(HashMap::new)
    }

    fn save_all(&self, map: &HashMap<String, Vec<String>>) -> Result<(), AppError> {
        let content = match serde_json::to_string_pretty(map) {
            Ok(val) => val,
            Err(e) => return Err(AppError::Serde(e)),
        };
        crate::persist::atomic_write(&self.path, &content)
    }

    pub fn get(&self, key: &str) -> Vec<String> {
        let map = self.load_all();
        match map.get(key) {
            Some(commands) => commands.clone(),
            None => Vec::new(),
        }
    }

    /// Append a command, moving any existing identical entry to the most-recent
    /// position and capping the list to the newest MAX_HISTORY commands.
    pub fn push(&self, key: &str, command: String) -> Result<(), AppError> {
        let _guard = match self.lock.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let mut map = self.load_all();
        let commands = map.entry(key.to_string()).or_insert_with(Vec::new);
        commands.retain(|existing| existing != &command);
        commands.push(command);
        if commands.len() > MAX_HISTORY {
            let overflow = commands.len() - MAX_HISTORY;
            commands.drain(0..overflow);
        }
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
