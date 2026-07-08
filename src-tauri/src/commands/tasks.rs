// Commands backing the Tasks panel. Step 1 covers create/read/update/delete of
// task definitions; the run engine (manual run + the scheduler's shared entry
// point) and the run-log queries are added in later steps.

use crate::error::AppError;
use crate::tasks::{now_ms, TaskDef, TaskRunStore, TaskStore};
use tauri::State;
use uuid::Uuid;

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
