use crate::error::AppError;
use crate::pool::ConnectionPool;
use crate::storage::Storage;
use mongodb::bson;
use tauri::State;

// Map a UI "kind" to the admin diagnostic command to run. Pure, so it's
// unit-tested; an unknown kind is rejected rather than sending a bogus command.
pub(crate) fn info_command(kind: &str) -> Result<bson::Document, String> {
    match kind {
        "build" => Ok(bson::doc! { "buildInfo": 1 }),
        "host" => Ok(bson::doc! { "hostInfo": 1 }),
        "replica" => Ok(bson::doc! { "replSetGetStatus": 1 }),
        other => Err(format!("Unknown server info kind: {other}")),
    }
}

/// Run a read-only server diagnostic on the admin database and return the raw
/// result. `kind` is one of "build" (buildInfo), "host" (hostInfo), or "replica"
/// (replSetGetStatus). Mirrors the extra entries in Studio-3T's Server Info menu.
#[tauri::command]
pub async fn server_info(
    pool: State<'_, ConnectionPool>,
    storage: State<'_, Storage>,
    id: String,
    kind: String,
) -> Result<serde_json::Value, AppError> {
    let config = match storage.find(&id) {
        Some(val) => val,
        None => return Err(AppError::UnknownConnection(id)),
    };
    let command = match info_command(&kind) {
        Ok(val) => val,
        Err(e) => return Err(AppError::Bson(e)),
    };
    let password = crate::keychain::get(&id);
    let client = match pool.connect(&config, password.as_deref()).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let result = match client.database("admin").run_command(command).await {
        Ok(val) => val,
        Err(e) => return Err(AppError::Mongo(e)),
    };
    Ok(serde_json::Value::from(bson::Bson::Document(result)))
}

#[cfg(test)]
mod tests;
