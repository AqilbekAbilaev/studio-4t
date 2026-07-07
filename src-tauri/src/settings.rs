use crate::error::AppError;
use crate::json_store::JsonStore;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
    inner: JsonStore<Settings>,
}

impl SettingsStorage {
    pub fn new(path: PathBuf) -> Self {
        Self { inner: JsonStore::new(path) }
    }

    pub fn load(&self) -> Settings {
        self.inner.load()
    }

    pub fn save(&self, settings: &Settings) -> Result<(), AppError> {
        self.inner.save(settings)
    }
}
