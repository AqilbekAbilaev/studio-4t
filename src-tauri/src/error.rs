use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("MongoDB error: {0}")]
    Mongo(#[from] mongodb::error::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("Cannot reach {address}: {reason}")]
    Unreachable { address: String, reason: String },

    #[error("BSON error: {0}")]
    Bson(String),
}

// Tauri commands return Result<T, E> where E must implement serde::Serialize.
// We serialize as a plain string so the frontend receives the human-readable message.
impl serde::Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}
