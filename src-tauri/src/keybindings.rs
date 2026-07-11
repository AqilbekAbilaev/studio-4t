use crate::error::AppError;
use crate::json_store::JsonStore;
use std::collections::HashMap;
use std::path::PathBuf;

/// Persisted keyboard-shortcut overrides, keyed by menu-action id
/// (e.g. "file:connect") with a Tauri accelerator string as the value
/// (e.g. "CmdOrCtrl+N", "F4"). This is the single source of truth the native
/// menu (menu.rs) and the frontend JS key handler both read, so a rebind stays
/// consistent across the menu bar and the in-window shortcuts.
///
/// Only ids the frontend knows about are ever written; an empty map means
/// "use the built-in defaults everywhere".
pub struct KeybindingStorage {
    inner: JsonStore<HashMap<String, String>>,
}

impl KeybindingStorage {
    pub fn new(path: PathBuf) -> Self {
        Self { inner: JsonStore::new(path) }
    }

    pub fn load(&self) -> HashMap<String, String> {
        self.inner.load()
    }

    /// Replace the whole binding map. The frontend sends the full effective set
    /// (defaults + user changes) so the file is self-contained.
    pub fn save(&self, bindings: &HashMap<String, String>) -> Result<(), AppError> {
        self.inner.save(bindings)
    }
}
