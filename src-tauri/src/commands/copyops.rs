use crate::error::AppError;
use crate::pool::ConnectionPool;
use crate::storage::Storage;
use mongodb::bson;
use tauri::State;

use super::client_for;

/// Copy a collection to another collection (optionally in another database) on the
/// SAME connection, via an aggregation `$out`. The target collection is replaced if
/// it already exists. Cross-connection copies are not supported here (the frontend
/// guards against them).
#[tauri::command]
pub async fn copy_collection(
    pool: State<'_, ConnectionPool>,
    storage: State<'_, Storage>,
    id: String,
    source_database: String,
    source_collection: String,
    target_database: String,
    target_collection: String,
) -> Result<(), AppError> {
    let client = match client_for(&pool, &storage, &id).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let src = client
        .database(&source_database)
        .collection::<bson::Document>(&source_collection);
    let pipeline = vec![
        bson::doc! { "$match": {} },
        bson::doc! { "$out": { "db": &target_database, "coll": &target_collection } },
    ];
    // $out runs as the aggregation is driven, so advance the cursor to completion.
    let mut cursor = match src.aggregate(pipeline).await {
        Ok(val) => val,
        Err(e) => return Err(AppError::Mongo(e)),
    };
    loop {
        match cursor.advance().await {
            Ok(true) => continue,
            Ok(false) => break,
            Err(e) => return Err(AppError::Mongo(e)),
        }
    }
    Ok(())
}
