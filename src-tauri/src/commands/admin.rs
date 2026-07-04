use crate::error::AppError;
use crate::pool::ConnectionPool;
use crate::storage::Storage;
use mongodb::bson;
use mongodb::options::IndexOptions;
use mongodb::IndexModel;
use tauri::State;

use super::{csv_to_docs, docs_to_csv, parse_ejson_document, parse_json_documents, DatabaseInfo};

/// The `_id_` index is created and required by MongoDB and can never be dropped,
/// hidden, or otherwise modified. The index-management guards share this check so
/// the rule lives in one place (kept pure so it can be unit-tested).
pub fn is_protected_index(name: &str) -> bool {
    name == "_id_"
}

#[tauri::command]
pub async fn list_databases(
    pool: State<'_, ConnectionPool>,
    storage: State<'_, Storage>,
    id: String,
) -> Result<Vec<DatabaseInfo>, AppError> {
    let config = match storage.find(&id) {
        Some(val) => val,
        None => return Err(AppError::UnknownConnection(id)),
    };
    let password = crate::keychain::get(&id);
    let client = match pool.connect(&config, password.as_deref()).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };

    let db_names = match client.list_database_names().await {
        Ok(val) => val,
        Err(e) => return Err(AppError::Mongo(e)),
    };
    let mut databases = Vec::new();
    for name in db_names {
        let collections = match client.database(&name).list_collection_names().await {
            Ok(val) => val,
            Err(_) => {
                databases.push(DatabaseInfo {
                    name: name,
                    collections: Vec::new(),
                    accessible: false,
                });
                continue;
            }
        };
        databases.push(DatabaseInfo {
            name: name,
            collections: collections,
            accessible: true,
        });
    }
    Ok(databases)
}

#[tauri::command]
pub async fn create_collection(
    pool: State<'_, ConnectionPool>,
    storage: State<'_, Storage>,
    id: String,
    database: String,
    name: String,
) -> Result<(), AppError> {
    let config = match storage.find(&id) {
        Some(val) => val,
        None => return Err(AppError::UnknownConnection(id)),
    };
    let password = crate::keychain::get(&id);
    let client = match pool.connect(&config, password.as_deref()).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    match client.database(&database).create_collection(&name).await {
        Ok(val) => Ok(val),
        Err(e) => Err(AppError::Mongo(e)),
    }
}

#[tauri::command]
pub async fn drop_database(
    pool: State<'_, ConnectionPool>,
    storage: State<'_, Storage>,
    id: String,
    database: String,
) -> Result<(), AppError> {
    let config = match storage.find(&id) {
        Some(val) => val,
        None => return Err(AppError::UnknownConnection(id)),
    };
    let password = crate::keychain::get(&id);
    let client = match pool.connect(&config, password.as_deref()).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    match client.database(&database).drop().await {
        Ok(val) => Ok(val),
        Err(e) => Err(AppError::Mongo(e)),
    }
}

#[tauri::command]
pub async fn drop_collection(
    pool: State<'_, ConnectionPool>,
    storage: State<'_, Storage>,
    id: String,
    database: String,
    collection: String,
) -> Result<(), AppError> {
    let config = match storage.find(&id) {
        Some(val) => val,
        None => return Err(AppError::UnknownConnection(id)),
    };
    let password = crate::keychain::get(&id);
    let client = match pool.connect(&config, password.as_deref()).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let col = client
        .database(&database)
        .collection::<bson::Document>(&collection);
    match col.drop().await {
        Ok(val) => Ok(val),
        Err(e) => Err(AppError::Mongo(e)),
    }
}

#[tauri::command]
pub async fn rename_collection(
    pool: State<'_, ConnectionPool>,
    storage: State<'_, Storage>,
    id: String,
    database: String,
    collection: String,
    new_name: String,
) -> Result<(), AppError> {
    let config = match storage.find(&id) {
        Some(val) => val,
        None => return Err(AppError::UnknownConnection(id)),
    };
    let password = crate::keychain::get(&id);
    let client = match pool.connect(&config, password.as_deref()).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    // MongoDB has no per-collection rename helper; the admin `renameCollection`
    // command takes fully-qualified `db.collection` namespaces for both sides.
    let from_namespace = format!("{}.{}", database, collection);
    let to_namespace = format!("{}.{}", database, new_name);
    let command = bson::doc! {
        "renameCollection": from_namespace,
        "to": to_namespace,
    };
    match client.database("admin").run_command(command).await {
        Ok(_) => Ok(()),
        Err(e) => Err(AppError::Mongo(e)),
    }
}

#[tauri::command]
pub async fn create_database(
    pool: State<'_, ConnectionPool>,
    storage: State<'_, Storage>,
    id: String,
    database: String,
    first_collection: String,
) -> Result<(), AppError> {
    let config = match storage.find(&id) {
        Some(val) => val,
        None => return Err(AppError::UnknownConnection(id)),
    };
    let password = crate::keychain::get(&id);
    let client = match pool.connect(&config, password.as_deref()).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    // A MongoDB database only materializes once it holds content, so creating the
    // first collection is what actually brings the database into existence.
    match client
        .database(&database)
        .create_collection(&first_collection)
        .await
    {
        Ok(val) => Ok(val),
        Err(e) => Err(AppError::Mongo(e)),
    }
}

/// Run admin `serverStatus` for a connection — host, version, uptime, current
/// connections, memory, etc. Returned raw as JSON; the frontend surfaces the
/// headline fields it cares about.
#[tauri::command]
pub async fn server_status(
    pool: State<'_, ConnectionPool>,
    storage: State<'_, Storage>,
    id: String,
) -> Result<serde_json::Value, AppError> {
    let config = match storage.find(&id) {
        Some(val) => val,
        None => return Err(AppError::UnknownConnection(id)),
    };
    let password = crate::keychain::get(&id);
    let client = match pool.connect(&config, password.as_deref()).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let command = bson::doc! { "serverStatus": 1 };
    let result = match client.database("admin").run_command(command).await {
        Ok(val) => val,
        Err(e) => return Err(AppError::Mongo(e)),
    };
    Ok(serde_json::Value::from(bson::Bson::Document(result)))
}

#[tauri::command]
pub async fn list_indexes(
    pool: State<'_, ConnectionPool>,
    storage: State<'_, Storage>,
    id: String,
    database: String,
    collection: String,
) -> Result<Vec<serde_json::Value>, AppError> {
    let config = match storage.find(&id) {
        Some(val) => val,
        None => return Err(AppError::UnknownConnection(id)),
    };
    let password = crate::keychain::get(&id);
    let client = match pool.connect(&config, password.as_deref()).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    // `listIndexes` returns the raw index documents (key spec, name, unique, …)
    // inside a cursor envelope; the frontend only needs the first batch to display.
    let command = bson::doc! { "listIndexes": collection };
    let result = match client.database(&database).run_command(command).await {
        Ok(val) => val,
        Err(e) => return Err(AppError::Mongo(e)),
    };
    let cursor_doc = match result.get_document("cursor") {
        Ok(val) => val,
        Err(e) => return Err(AppError::Bson(e.to_string())),
    };
    let first_batch = match cursor_doc.get_array("firstBatch") {
        Ok(val) => val,
        Err(e) => return Err(AppError::Bson(e.to_string())),
    };
    let mut indexes = Vec::new();
    for entry in first_batch {
        indexes.push(serde_json::Value::from(entry.clone()));
    }
    Ok(indexes)
}

#[tauri::command]
pub async fn create_index(
    pool: State<'_, ConnectionPool>,
    storage: State<'_, Storage>,
    id: String,
    database: String,
    collection: String,
    keys: String,
    unique: bool,
    name: String,
) -> Result<(), AppError> {
    let config = match storage.find(&id) {
        Some(val) => val,
        None => return Err(AppError::UnknownConnection(id)),
    };
    let password = crate::keychain::get(&id);
    let client = match pool.connect(&config, password.as_deref()).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let col = client
        .database(&database)
        .collection::<bson::Document>(&collection);
    let keys_doc = match parse_ejson_document(&keys) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    // An empty name lets MongoDB auto-generate one from the key spec.
    let index_options = if name.trim().is_empty() {
        IndexOptions::builder().unique(Some(unique)).build()
    } else {
        IndexOptions::builder()
            .unique(Some(unique))
            .name(Some(name))
            .build()
    };
    let model = IndexModel::builder()
        .keys(keys_doc)
        .options(Some(index_options))
        .build();
    match col.create_index(model).await {
        Ok(_) => Ok(()),
        Err(e) => Err(AppError::Mongo(e)),
    }
}

#[tauri::command]
pub async fn drop_index(
    pool: State<'_, ConnectionPool>,
    storage: State<'_, Storage>,
    id: String,
    database: String,
    collection: String,
    name: String,
) -> Result<(), AppError> {
    // The `_id_` index cannot be dropped; reject it before touching the network.
    if is_protected_index(&name) {
        return Err(AppError::Validation(format!(
            "The \"{}\" index cannot be dropped.",
            name
        )));
    }
    let config = match storage.find(&id) {
        Some(val) => val,
        None => return Err(AppError::UnknownConnection(id)),
    };
    let password = crate::keychain::get(&id);
    let client = match pool.connect(&config, password.as_deref()).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let col = client
        .database(&database)
        .collection::<bson::Document>(&collection);
    match col.drop_index(name).await {
        Ok(_) => Ok(()),
        Err(e) => Err(AppError::Mongo(e)),
    }
}

/// Sets or clears an index's `hidden` flag via `collMod`. A hidden index is
/// ignored by the query planner but kept up to date, so it can be un-hidden
/// instantly without a rebuild. The `_id_` index cannot be hidden.
#[tauri::command]
pub async fn set_index_hidden(
    pool: State<'_, ConnectionPool>,
    storage: State<'_, Storage>,
    id: String,
    database: String,
    collection: String,
    name: String,
    hidden: bool,
) -> Result<(), AppError> {
    if is_protected_index(&name) {
        return Err(AppError::Validation(format!(
            "The \"{}\" index cannot be hidden.",
            name
        )));
    }
    let config = match storage.find(&id) {
        Some(val) => val,
        None => return Err(AppError::UnknownConnection(id)),
    };
    let password = crate::keychain::get(&id);
    let client = match pool.connect(&config, password.as_deref()).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let command = bson::doc! {
        "collMod": &collection,
        "index": { "name": &name, "hidden": hidden },
    };
    match client.database(&database).run_command(command).await {
        Ok(_) => Ok(()),
        Err(e) => Err(AppError::Mongo(e)),
    }
}

/// Returns the `$indexStats` usage entries for a collection (one per index, with
/// access counts and the time tracking began). Used by the index "View Details"
/// view; the frontend matches entries to indexes by name. Callers treat an error
/// as "stats unavailable" (e.g. on a server/deployment that doesn't support it).
#[tauri::command]
pub async fn index_stats(
    pool: State<'_, ConnectionPool>,
    storage: State<'_, Storage>,
    id: String,
    database: String,
    collection: String,
) -> Result<Vec<serde_json::Value>, AppError> {
    let config = match storage.find(&id) {
        Some(val) => val,
        None => return Err(AppError::UnknownConnection(id)),
    };
    let password = crate::keychain::get(&id);
    let client = match pool.connect(&config, password.as_deref()).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let col = client
        .database(&database)
        .collection::<bson::Document>(&collection);
    let pipeline = vec![bson::doc! { "$indexStats": {} }];
    let mut cursor = match col.aggregate(pipeline).await {
        Ok(val) => val,
        Err(e) => return Err(AppError::Mongo(e)),
    };
    let mut stats = Vec::new();
    loop {
        let has_next = match cursor.advance().await {
            Ok(val) => val,
            Err(e) => return Err(AppError::Mongo(e)),
        };
        if !has_next {
            break;
        }
        let entry: bson::Document = match cursor.deserialize_current() {
            Ok(val) => val,
            Err(e) => return Err(AppError::Mongo(e)),
        };
        stats.push(serde_json::Value::from(bson::Bson::Document(entry)));
    }
    Ok(stats)
}

#[tauri::command]
pub async fn export_collection(
    pool: State<'_, ConnectionPool>,
    storage: State<'_, Storage>,
    id: String,
    database: String,
    collection: String,
    path: String,
    format: String,
) -> Result<usize, AppError> {
    let config = match storage.find(&id) {
        Some(val) => val,
        None => return Err(AppError::UnknownConnection(id)),
    };
    let password = crate::keychain::get(&id);
    let client = match pool.connect(&config, password.as_deref()).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let col = client
        .database(&database)
        .collection::<bson::Document>(&collection);
    let mut cursor = match col.find(bson::doc! {}).await {
        Ok(val) => val,
        Err(e) => return Err(AppError::Mongo(e)),
    };
    let mut docs = Vec::new();
    loop {
        let has_next = match cursor.advance().await {
            Ok(val) => val,
            Err(e) => return Err(AppError::Mongo(e)),
        };
        if !has_next {
            break;
        }
        let doc: bson::Document = match cursor.deserialize_current() {
            Ok(val) => val,
            Err(e) => return Err(AppError::Mongo(e)),
        };
        docs.push(doc);
    }
    let count = docs.len();

    let contents = if format == "csv" {
        docs_to_csv(&docs)
    } else {
        let values: Vec<serde_json::Value> = docs
            .into_iter()
            .map(|doc| serde_json::Value::from(bson::Bson::Document(doc)))
            .collect();
        match serde_json::to_string_pretty(&values) {
            Ok(val) => val,
            Err(e) => return Err(AppError::Serde(e)),
        }
    };
    match std::fs::write(&path, contents) {
        Ok(_) => Ok(count),
        Err(e) => Err(AppError::Io(e)),
    }
}

#[tauri::command]
pub async fn import_collection(
    pool: State<'_, ConnectionPool>,
    storage: State<'_, Storage>,
    id: String,
    database: String,
    collection: String,
    path: String,
    format: String,
) -> Result<usize, AppError> {
    let contents = match std::fs::read_to_string(&path) {
        Ok(val) => val,
        Err(e) => return Err(AppError::Io(e)),
    };
    let docs = if format == "csv" {
        csv_to_docs(&contents)
    } else {
        match parse_json_documents(&contents) {
            Ok(val) => val,
            Err(e) => return Err(e),
        }
    };
    if docs.is_empty() {
        return Ok(0);
    }

    let config = match storage.find(&id) {
        Some(val) => val,
        None => return Err(AppError::UnknownConnection(id)),
    };
    let password = crate::keychain::get(&id);
    let client = match pool.connect(&config, password.as_deref()).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let col = client
        .database(&database)
        .collection::<bson::Document>(&collection);
    match col.insert_many(docs).await {
        Ok(result) => Ok(result.inserted_ids.len()),
        Err(e) => Err(AppError::Mongo(e)),
    }
}
