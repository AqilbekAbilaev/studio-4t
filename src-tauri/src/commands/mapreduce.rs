use crate::error::AppError;
use crate::pool::ConnectionPool;
use crate::storage::Storage;
use mongodb::bson;
use tauri::State;

use super::client_for;

/// Run a `mapReduce` over a collection. `map`/`reduce` (and optional `finalize`) are
/// JavaScript function sources. An empty `out_collection` runs inline (results
/// returned); otherwise results are written to that collection. The raw result
/// document is returned as JSON.
#[tauri::command]
pub async fn map_reduce(
    pool: State<'_, ConnectionPool>,
    storage: State<'_, Storage>,
    id: String,
    database: String,
    collection: String,
    map: String,
    reduce: String,
    finalize: String,
    out_collection: String,
) -> Result<serde_json::Value, AppError> {
    let client = match client_for(&pool, &storage, &id).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let out: bson::Bson = if out_collection.trim().is_empty() {
        bson::Bson::Document(bson::doc! { "inline": 1 })
    } else {
        bson::Bson::String(out_collection.trim().to_string())
    };
    let mut command = bson::doc! {
        "mapReduce": &collection,
        "map": bson::Bson::JavaScriptCode(map),
        "reduce": bson::Bson::JavaScriptCode(reduce),
        "out": out,
    };
    if !finalize.trim().is_empty() {
        command.insert("finalize", bson::Bson::JavaScriptCode(finalize));
    }
    let result = match client.database(&database).run_command(command).await {
        Ok(val) => val,
        Err(e) => return Err(AppError::Mongo(e)),
    };
    Ok(serde_json::Value::from(bson::Bson::Document(result)))
}
