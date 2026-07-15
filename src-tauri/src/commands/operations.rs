use crate::error::AppError;
use crate::operations::{OperationRecord, OperationsRegistry};
use tauri::State;

/// The Operations pane's data source: the current union of running + persisted
/// operations, newest first.
#[tauri::command]
pub async fn list_operations(
    ops: State<'_, OperationsRegistry>,
) -> Result<Vec<OperationRecord>, AppError> {
    Ok(ops.list())
}

/// Clear the persisted terminal log (running operations are left alone).
#[tauri::command]
pub async fn clear_operations(ops: State<'_, OperationsRegistry>) -> Result<(), AppError> {
    ops.clear_finished();
    Ok(())
}
