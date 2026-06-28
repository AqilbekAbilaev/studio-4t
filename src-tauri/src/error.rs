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

    #[error("Unknown connection: {0}")]
    UnknownConnection(String),

    #[error("BSON error: {0}")]
    Bson(String),

    #[error("Keychain error: {0}")]
    Keychain(String),

    #[error("Shell error: {0}")]
    Shell(String),

    #[error("SSH tunnel error: {0}")]
    Ssh(String),
}

impl AppError {
    /// Stable category for logging (and a future structured-error wire format).
    pub fn code(&self) -> &'static str {
        match self {
            AppError::Mongo(_) => "mongo",
            AppError::Io(_) => "io",
            AppError::Serde(_) => "serde",
            AppError::Unreachable { .. } => "unreachable",
            AppError::UnknownConnection(_) => "unknown_connection",
            AppError::Bson(_) => "bson",
            AppError::Keychain(_) => "keychain",
            AppError::Shell(_) => "shell",
            AppError::Ssh(_) => "ssh",
        }
    }
}

// Tauri commands return Result<T, E> where E must implement serde::Serialize.
// We serialize as a plain string so the frontend receives the human-readable
// message (unchanged contract). This is also the single funnel through which
// every error returned to the frontend passes, so we log it here (with its
// category) for diagnosis.
impl serde::Serialize for AppError {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        eprintln!("[studio-4t] error [{}]: {}", self.code(), self);
        s.serialize_str(&self.to_string())
    }
}
