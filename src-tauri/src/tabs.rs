use crate::error::AppError;
use std::path::PathBuf;

pub struct TabStorage {
    path: PathBuf,
}

impl TabStorage {
    pub fn new(path: PathBuf) -> Self {
        Self { path: path }
    }

    pub fn load(&self) -> Option<serde_json::Value> {
        if !self.path.exists() {
            return None;
        }
        let content = match std::fs::read_to_string(&self.path) {
            Ok(val) => val,
            Err(_)  => return None,
        };
        match serde_json::from_str(&content) {
            Ok(val) => Some(val),
            Err(_)  => None,
        }
    }

    pub fn save(&self, session: &serde_json::Value) -> Result<(), AppError> {
        let content = match serde_json::to_string_pretty(session) {
            Ok(val) => val,
            Err(e)  => return Err(AppError::Serde(e)),
        };
        crate::persist::atomic_write(&self.path, &content)
    }
}
