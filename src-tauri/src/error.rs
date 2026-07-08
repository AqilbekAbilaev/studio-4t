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

    #[error("This connection is read-only. Writes are disabled for \"{name}\".")]
    ReadOnly { name: String },

    #[error("BSON error: {0}")]
    Bson(String),

    #[error("Keychain error: {0}")]
    Keychain(String),

    #[error("Shell error: {0}")]
    Shell(String),

    #[error("SSH tunnel error: {0}")]
    Ssh(String),

    #[error("SQL error: {0}")]
    Sql(String),

    #[error("{0}")]
    Validation(String),
}

impl AppError {
    /// Stable category for logging and the structured-error wire format. The
    /// frontend branches on this (e.g. to show an auth vs network hint), so
    /// Mongo errors are classified into actionable sub-categories.
    pub fn code(&self) -> &'static str {
        match self {
            AppError::Mongo(e) => match e.kind.as_ref() {
                mongodb::error::ErrorKind::Authentication { .. } => "auth",
                mongodb::error::ErrorKind::InvalidTlsConfig { .. } => "tls",
                mongodb::error::ErrorKind::ServerSelection { .. }
                | mongodb::error::ErrorKind::DnsResolve { .. }
                | mongodb::error::ErrorKind::ConnectionPoolCleared { .. }
                | mongodb::error::ErrorKind::Io(_) => "network",
                _ => "mongo",
            },
            AppError::Io(_) => "io",
            AppError::Serde(_) => "serde",
            AppError::Unreachable { .. } => "unreachable",
            AppError::UnknownConnection(_) => "unknown_connection",
            AppError::ReadOnly { .. } => "read_only",
            AppError::Bson(_) => "bson",
            AppError::Keychain(_) => "keychain",
            AppError::Shell(_) => "shell",
            AppError::Ssh(_) => "ssh",
            AppError::Sql(_) => "sql",
            AppError::Validation(_) => "validation",
        }
    }
}

// Tauri commands return Result<T, E> where E must implement serde::Serialize.
// We serialize as { code, message } so the frontend gets both a stable category
// to branch on and a human-readable message. This is also the single funnel
// through which every error returned to the frontend passes, so we log it here
// (with its category) for diagnosis.
#[derive(serde::Serialize)]
struct WireError<'a> {
    code: &'a str,
    message: String,
}

impl serde::Serialize for AppError {
    // This is the deliberate single funnel: every error returned to the frontend
    // passes through here, so it's the one place we log. Gate the log by category —
    // expected user-input errors (`validation`, `bson`) surface to the user as calm
    // toasts, so logging them here is just noise. Everything else is genuinely
    // unexpected / server-side, so we log it for diagnosis.
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let code = self.code();
        match code {
            "validation" | "bson" => {}
            _ => eprintln!("[studio-4t] error [{}]: {}", code, self),
        }
        let wire = WireError {
            code: code,
            message: self.to_string(),
        };
        wire.serialize(s)
    }
}
