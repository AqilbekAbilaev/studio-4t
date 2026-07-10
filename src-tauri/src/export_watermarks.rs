use crate::error::AppError;
use crate::json_store::JsonStore;
use std::collections::HashMap;
use std::path::PathBuf;

/// Persisted "high-water marks" for incremental export: the largest `_id` already
/// exported for a collection, so a later incremental export only writes documents added
/// since. Keyed by tree path "connId/dbName/collName"; the value is the boundary `_id`
/// as a canonical Extended-JSON string (so any `_id` type — ObjectId, int, string —
/// round-trips). An absent entry means "never exported incrementally" → export everything.
pub struct ExportWatermarkStorage {
    inner: JsonStore<HashMap<String, String>>,
}

impl ExportWatermarkStorage {
    pub fn new(path: PathBuf) -> Self {
        Self { inner: JsonStore::new(path) }
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.inner.load().get(key).cloned()
    }

    pub fn set(&self, key: &str, watermark: &str) -> Result<(), AppError> {
        self.inner.update(|map| {
            map.insert(key.to_string(), watermark.to_string());
        })
    }
}
