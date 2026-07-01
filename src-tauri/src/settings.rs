use crate::error::AppError;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

/// Application-wide preferences. A single JSON object (not keyed), persisted to
/// `settings.json`. New fields should carry `#[serde(default)]` so older files
/// still deserialize.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Settings {
    #[serde(default = "default_query_limit")]
    pub default_query_limit: i64,
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_query_limit() -> i64 {
    50
}

fn default_theme() -> String {
    "dark".to_string()
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            default_query_limit: default_query_limit(),
            theme: default_theme(),
        }
    }
}

pub struct SettingsStorage {
    path: PathBuf,
    lock: Mutex<()>,
}

impl SettingsStorage {
    pub fn new(path: PathBuf) -> Self {
        Self { path: path, lock: Mutex::new(()) }
    }

    pub fn load(&self) -> Settings {
        if !self.path.exists() {
            return Settings::default();
        }
        let content = std::fs::read_to_string(&self.path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_default()
    }

    pub fn save(&self, settings: &Settings) -> Result<(), AppError> {
        let _guard = match self.lock.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let content = match serde_json::to_string_pretty(settings) {
            Ok(val) => val,
            Err(e) => return Err(AppError::Serde(e)),
        };
        crate::persist::atomic_write(&self.path, &content)
    }
}
