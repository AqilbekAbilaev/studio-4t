use crate::error::AppError;
use crate::pool::ConnectionPool;
use crate::storage::Storage;
use mongodb::bson;
use tauri::State;

// Validate a duplicate target name before touching the server: non-empty, distinct
// from the source, and not colliding with an existing collection (so we never
// clobber data). Pure, so it's unit-tested directly.
pub(crate) fn validate_target(source: &str, target: &str, existing: &[String]) -> Result<(), String> {
    let trimmed = target.trim();
    if trimmed.is_empty() {
        return Err("A target collection name is required".to_string());
    }
    if trimmed == source {
        return Err("The copy must have a different name than the source".to_string());
    }
    if existing.iter().any(|name| name == trimmed) {
        return Err(format!("A collection named '{trimmed}' already exists"));
    }
    Ok(())
}

/// Duplicate a collection within the same database: copies every document into a
/// new collection via a server-side `$out` aggregation. Refuses to overwrite an
/// existing collection. Returns the number of documents copied. Note: indexes
/// other than `_id` are not carried over (a `$out` limitation).
#[tauri::command]
pub async fn duplicate_collection(
    pool: State<'_, ConnectionPool>,
    storage: State<'_, Storage>,
    id: String,
    database: String,
    source: String,
    target: String,
) -> Result<u64, AppError> {
    let config = match storage.find(&id) {
        Some(val) => val,
        None => return Err(AppError::UnknownConnection(id)),
    };
    let password = crate::keychain::get(&id);
    let client = match pool.connect(&config, password.as_deref()).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let db = client.database(&database);

    let existing = match db.list_collection_names().await {
        Ok(val) => val,
        Err(e) => return Err(AppError::Mongo(e)),
    };
    let target_name = target.trim().to_string();
    match validate_target(&source, &target_name, &existing) {
        Ok(_) => {}
        Err(e) => return Err(AppError::Bson(e)),
    }

    // Copy documents server-side. `$out` writes the results into the target
    // collection; the returned cursor yields nothing, so we just drive it to
    // completion to ensure the write has happened.
    let col = db.collection::<bson::Document>(&source);
    let pipeline = vec![
        bson::doc! { "$match": {} },
        bson::doc! { "$out": &target_name },
    ];
    let mut cursor = match col.aggregate(pipeline).await {
        Ok(val) => val,
        Err(e) => return Err(AppError::Mongo(e)),
    };
    loop {
        let has_next = match cursor.advance().await {
            Ok(val) => val,
            Err(e) => return Err(AppError::Mongo(e)),
        };
        if !has_next {
            break;
        }
    }

    let target_col = db.collection::<bson::Document>(&target_name);
    match target_col.count_documents(bson::doc! {}).await {
        Ok(val) => Ok(val),
        Err(e) => Err(AppError::Mongo(e)),
    }
}

#[cfg(test)]
mod tests;
