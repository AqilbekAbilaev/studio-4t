// ── IntelliShell (embedded JavaScript shell) ──────────────────────────────

use crate::error::AppError;
use crate::pool::ConnectionPool;
use crate::shell::{ShellEngine, ShellResult};
use crate::shell_history::ShellHistoryStorage;
use crate::storage::Storage;
use tauri::State;

/// Evaluate a block of JavaScript in the shell session identified by
/// `session_id`. Each session has its own persistent JS context, so variables
/// declared in one submission are visible in the next. The `db` global is bound
/// to `id`'s connection and `database`. Returns the transcript (printed lines,
/// completion value, or a JS error message).
#[tauri::command]
pub async fn run_shell_command(
    pool: State<'_, ConnectionPool>,
    storage: State<'_, Storage>,
    shell: State<'_, ShellEngine>,
    id: String,
    database: String,
    session_id: String,
    code: String,
) -> Result<ShellResult, AppError> {
    // Resolve the connection exactly like find_documents so the shell shares the
    // same pooled client and credential flow.
    let config = match storage.find(&id) {
        Some(val) => val,
        None => return Err(AppError::UnknownConnection(id)),
    };
    let password = crate::keychain::get(&id);
    let client = match pool.connect(&config, password.as_deref()).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };

    let handle = tokio::runtime::Handle::current();
    let receiver = shell.submit_eval(session_id, code, client, database, handle);
    match receiver.await {
        Ok(result) => Ok(result),
        Err(_) => Err(AppError::Shell(String::from(
            "the shell engine is unavailable",
        ))),
    }
}

/// Drop a shell session's JavaScript context (called when its tab is closed).
#[tauri::command]
pub async fn close_shell_session(
    shell: State<'_, ShellEngine>,
    session_id: String,
) -> Result<(), AppError> {
    shell.close(session_id);
    Ok(())
}

/// Persisted IntelliShell command history for a connection (oldest first).
#[tauri::command]
pub fn get_shell_history(
    history: State<'_, ShellHistoryStorage>,
    connection_id: String,
) -> Vec<String> {
    history.get(&connection_id)
}

#[tauri::command]
pub fn push_shell_command(
    history: State<'_, ShellHistoryStorage>,
    connection_id: String,
    command: String,
) -> Result<(), AppError> {
    history.push(&connection_id, command)
}

#[tauri::command]
pub fn clear_shell_history(
    history: State<'_, ShellHistoryStorage>,
    connection_id: String,
) -> Result<(), AppError> {
    history.clear(&connection_id)
}
