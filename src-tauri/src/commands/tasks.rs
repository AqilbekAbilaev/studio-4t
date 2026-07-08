// Commands backing the Tasks panel: create/read/update/delete of task
// definitions, the run engine (`run_task_now`, shared by the manual `run_task`
// command and the in-app scheduler), and the per-task run log.

use crate::error::AppError;
use crate::shell::ShellEngine;
use crate::tasks::{now_ms, TaskDef, TaskRun, TaskRunStore, TaskSpec, TaskStore};
use tauri::{AppHandle, Manager, State};
use uuid::Uuid;

use super::AppContext;

/// All saved tasks, newest-first (list order as stored).
#[tauri::command]
pub fn list_tasks(store: State<'_, TaskStore>) -> Vec<TaskDef> {
    store.load()
}

/// Create or update a task. A blank id means "create": we mint a fresh uuid and
/// stamp `created_at`. A non-blank id updates the existing task in place.
/// Returns the task's id either way.
#[tauri::command]
pub fn save_task(store: State<'_, TaskStore>, task: TaskDef) -> Result<String, AppError> {
    let mut task = task;
    if task.id.trim().is_empty() {
        task.id = Uuid::new_v4().to_string();
        task.created_at = now_ms();
    }
    let id = task.id.clone();
    match store.upsert(task) {
        Ok(()) => Ok(id),
        Err(e) => Err(e),
    }
}

/// Delete a task and its run log.
#[tauri::command]
pub fn delete_task(
    store: State<'_, TaskStore>,
    runs: State<'_, TaskRunStore>,
    id: String,
) -> Result<(), AppError> {
    match store.delete(&id) {
        Ok(()) => {}
        Err(e) => return Err(e),
    }
    runs.clear(&id)
}

/// Run a task once, on demand. Records the outcome in the task's run log and
/// stamps `last_run`/`last_status`, then returns the run so the UI can show it.
#[tauri::command]
pub async fn run_task(app: AppHandle, id: String) -> Result<TaskRun, AppError> {
    // Look the task up and drop the store handle before the run's `.await`.
    let task = {
        let store = app.state::<TaskStore>();
        match store.find(&id) {
            Some(value) => value,
            None => return Err(AppError::Validation(format!("no task with id {}", id))),
        }
    };
    let run = run_task_now(&app, &task).await;
    match app.state::<TaskRunStore>().push(&id, run.clone()) {
        Ok(()) => {}
        Err(e) => return Err(e),
    }
    match app
        .state::<TaskStore>()
        .record_run(&id, &run.ran_at, &run.status)
    {
        Ok(()) => {}
        Err(e) => return Err(e),
    }
    Ok(run)
}

/// A task's run log, newest-first.
#[tauri::command]
pub fn get_task_runs(runs: State<'_, TaskRunStore>, id: String) -> Vec<TaskRun> {
    runs.get(&id)
}

/// The single entry point that actually runs a task. It resolves managed state
/// from the `AppHandle` — so the scheduler (which only holds an `AppHandle`, not a
/// request `State`) and the manual `run_task` command share exactly one code path —
/// and dispatches to the existing operation command for the task's spec. It always
/// returns a `TaskRun`: a failure is captured as an "error" run rather than
/// propagated, so one bad run can never derail the scheduler loop.
pub async fn run_task_now(app: &AppHandle, task: &TaskDef) -> TaskRun {
    let outcome = execute_spec(app, task).await;
    let ran_at = now_ms();
    match outcome {
        Ok(message) => TaskRun {
            ran_at: ran_at,
            status: String::from("ok"),
            message: message,
        },
        Err(e) => TaskRun {
            ran_at: ran_at,
            status: String::from("error"),
            message: e.to_string(),
        },
    }
}

// Dispatch to the operation command matching the task's spec, forwarding the
// managed state each one needs. Returns a short human-readable success message
// for the run log (or the operation's error).
async fn execute_spec(app: &AppHandle, task: &TaskDef) -> Result<String, AppError> {
    let id = task.connection_id.clone();
    match &task.spec {
        TaskSpec::Export {
            database,
            collection,
            path,
            format,
        } => {
            let count = match super::export_collection(
                app.state::<AppContext>(),
                id,
                database.clone(),
                collection.clone(),
                path.clone(),
                format.clone(),
            )
            .await
            {
                Ok(value) => value,
                Err(e) => return Err(e),
            };
            Ok(format!("Exported {} document(s) to {}", count, path))
        }
        TaskSpec::Import {
            database,
            collection,
            path,
            format,
        } => {
            let count = match super::import_collection(
                app.state::<AppContext>(),
                id,
                database.clone(),
                collection.clone(),
                path.clone(),
                format.clone(),
            )
            .await
            {
                Ok(value) => value,
                Err(e) => return Err(e),
            };
            Ok(format!("Imported {} document(s) from {}", count, path))
        }
        TaskSpec::Masking {
            database,
            collection,
            filter,
            rules,
            path,
            format,
            limit,
        } => {
            let count = match super::export_masked_collection(
                app.state::<AppContext>(),
                id,
                database.clone(),
                collection.clone(),
                filter.clone(),
                rules.clone(),
                path.clone(),
                format.clone(),
                *limit,
            )
            .await
            {
                Ok(value) => value,
                Err(e) => return Err(e),
            };
            Ok(format!("Exported {} masked document(s) to {}", count, path))
        }
        TaskSpec::Migration {
            database,
            collection,
            table_name,
            limit,
            path,
        } => {
            let sql = match super::generate_sql_migration(
                app.state::<AppContext>(),
                id,
                database.clone(),
                collection.clone(),
                table_name.clone(),
                *limit,
            )
            .await
            {
                Ok(value) => value,
                Err(e) => return Err(e),
            };
            match write_text_file(path, &sql) {
                Ok(()) => {}
                Err(e) => return Err(e),
            }
            Ok(format!("Wrote SQL migration to {}", path))
        }
        TaskSpec::Shell { database, code } => {
            // A task has no live console tab, so run the script in a throwaway
            // session (fresh JS context) and drop it afterwards.
            let session_id = Uuid::new_v4().to_string();
            let result = super::run_shell_command(
                app.state::<AppContext>(),
                app.state::<ShellEngine>(),
                id,
                database.clone(),
                session_id.clone(),
                code.clone(),
            )
            .await;
            // Close the session whether the eval succeeded or failed.
            app.state::<ShellEngine>().close(session_id);
            let transcript = match result {
                Ok(value) => value,
                Err(e) => return Err(e),
            };
            match transcript.error {
                Some(message) => Err(AppError::Shell(message)),
                None => Ok(shell_summary(&transcript)),
            }
        }
    }
}

// Flatten a shell transcript (printed lines + completion value) into one message
// for the run log.
fn shell_summary(result: &crate::shell::ShellResult) -> String {
    let mut summary = result.logs.join("\n");
    if let Some(value) = &result.value {
        if !summary.is_empty() {
            summary.push('\n');
        }
        summary.push_str(&value.to_string());
    }
    if summary.is_empty() {
        return String::from("Script completed");
    }
    summary
}

// Write `contents` to `path`, mapping an I/O failure into an AppError. The task's
// destination is an arbitrary user path (not an app-data JSON file), so this uses
// a plain write rather than the app-data atomic_write helper.
fn write_text_file(path: &str, contents: &str) -> Result<(), AppError> {
    match std::fs::write(path, contents.as_bytes()) {
        Ok(()) => Ok(()),
        Err(e) => Err(AppError::Io(e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn write_text_file_writes_contents() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("out.sql");
        let path_str = path.to_str().unwrap();
        write_text_file(path_str, "CREATE TABLE t (id INT);").unwrap();
        let read_back = std::fs::read_to_string(&path).unwrap();
        assert_eq!(read_back, "CREATE TABLE t (id INT);");
    }

    #[test]
    fn write_text_file_errors_on_bad_path() {
        // A path whose parent directory does not exist fails as an I/O error.
        let result = write_text_file("/nonexistent-dir-xyz/out.sql", "x");
        assert!(result.is_err());
    }
}
