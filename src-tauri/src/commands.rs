use crate::error::AppError;
use crate::pool::ConnectionPool;
use crate::storage::{ConnectionConfig, Storage};
use crate::uri;
use mongodb::bson;
use mongodb::Client;
use serde::Serialize;
use tauri::State;
use uuid::Uuid;

#[derive(Serialize)]
pub struct DatabaseInfo {
    pub name: String,
    pub collections: Vec<String>,
    pub accessible: bool,
}

#[tauri::command]
pub async fn test_connection(uri: String) -> Result<(), AppError> {
    match uri::tcp_probe(&uri).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let client = match Client::with_uri_str(&uri::with_timeout(&uri)).await {
        Ok(val) => val,
        Err(e) => return Err(AppError::Mongo(e)),
    };
    match client.list_database_names().await {
        Ok(_) => {},
        Err(e) => return Err(AppError::Mongo(e)),
    };
    Ok(())
}

#[tauri::command]
pub async fn save_connection(
    storage: State<'_, Storage>,
    pool: State<'_, ConnectionPool>,
    name: String,
    uri: String,
) -> Result<String, AppError> {
    let id = Uuid::new_v4().to_string();
    match storage.add(ConnectionConfig {
        id: id.clone(),
        name: name,
        uri: uri.clone(),
        last_accessed: None,
        tag: None,
    }) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };

    // Create and cache the client immediately so the first expand is instant.
    let client = match Client::with_uri_str(&uri::with_timeout(&uri)).await {
        Ok(val) => val,
        Err(e) => return Err(AppError::Mongo(e)),
    };
    pool.insert(id.clone(), client).await;

    Ok(id)
}

#[tauri::command]
pub fn list_connections(storage: State<'_, Storage>) -> Vec<ConnectionConfig> {
    storage.load()
}

#[tauri::command]
pub async fn delete_connection(
    storage: State<'_, Storage>,
    pool: State<'_, ConnectionPool>,
    id: String,
) -> Result<(), AppError> {
    match storage.remove(&id) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    pool.remove(&id).await;
    Ok(())
}

#[tauri::command]
pub async fn disconnect(
    pool: State<'_, ConnectionPool>,
    id: String,
) -> Result<(), AppError> {
    pool.remove(&id).await;
    Ok(())
}

// macOS's system-wide "Smart Quotes" substitutes " and ' for curly equivalents
// at the OS text-input layer, before the keystroke ever reaches the web page —
// no HTML attribute on the input can suppress it. Normalize here so a query
// typed (or pasted from a rich-text source) with curly quotes still parses.
fn normalize_smart_quotes(value: &str) -> String {
    value
        .chars()
        .map(|c: char| match c {
            '\u{201C}' | '\u{201D}' => '"',
            '\u{2018}' | '\u{2019}' => '\'',
            other => other,
        })
        .collect()
}

fn parse_filter(filter: &str) -> Result<bson::Document, AppError> {
    let trimmed = filter.trim();
    if trimmed.is_empty() || trimmed == "{}" {
        return Ok(bson::doc! {});
    }
    let normalized = normalize_smart_quotes(trimmed);
    // Deserialize via bson::Bson so that extended-JSON types ({"$oid": "..."}, {"$date": "..."})
    // are correctly decoded into their BSON equivalents. serde_json::Value + bson::to_document
    // would treat {"$oid": "..."} as a plain nested document, breaking _id filters.
    let bson_val: bson::Bson = match serde_json::from_str(&normalized) {
        Ok(val) => val,
        Err(e) => return Err(AppError::Bson(format!(
            // serde_json's "key must be a string" fires on unquoted keys like { name: 1 }.
            // The frontend preprocesses these, so if we still hit this, surface a clear hint.
            "Invalid query JSON ({e}). Keys must be quoted, e.g. {{\"name\": 1}}"
        ))),
    };
    match bson_val {
        bson::Bson::Document(doc) => Ok(doc),
        _ => Err(AppError::Bson("Filter must be a JSON object".to_string())),
    }
}

#[tauri::command]
pub async fn find_documents(
    pool: State<'_, ConnectionPool>,
    id: String,
    uri: String,
    database: String,
    collection: String,
    filter: String,
    projection: String,
    sort: String,
    skip: i64,
    limit: i64,
) -> Result<Vec<serde_json::Value>, AppError> {
    let client = match pool.get_or_create(&id, &uri::with_timeout(&uri)).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let col = client
        .database(&database)
        .collection::<bson::Document>(&collection);

    let filter_doc = match parse_filter(&filter) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let projection_doc = match parse_filter(&projection) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let sort_doc = match parse_filter(&sort) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };

    let mut query = col.find(filter_doc).limit(limit).skip(skip as u64);
    if !projection_doc.is_empty() {
        query = query.projection(projection_doc);
    }
    if !sort_doc.is_empty() {
        query = query.sort(sort_doc);
    }

    let mut cursor = match query.await {
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
        // Use bson's own From impl (not serde_json::to_value) — bson's Serialize
        // targets the bson wire format, not JSON, so to_value produces wrong output.
        docs.push(serde_json::Value::from(bson::Bson::Document(doc)));
    }
    Ok(docs)
}

#[tauri::command]
pub fn set_connection_tag(
    storage: State<'_, Storage>,
    id: String,
    tag: String,
) -> Result<(), AppError> {
    let mut connections = storage.load();
    if let Some(c) = connections.iter_mut().find(|c| c.id == id) {
        c.tag = if tag.is_empty() { None } else { Some(tag) };
    }
    storage.save(&connections)
}

#[tauri::command]
pub fn update_last_accessed(
    storage: State<'_, Storage>,
    id: String,
    timestamp: String,
) -> Result<(), AppError> {
    let mut connections = storage.load();
    if let Some(c) = connections.iter_mut().find(|c| c.id == id) {
        c.last_accessed = Some(timestamp);
    }
    storage.save(&connections)
}

#[tauri::command]
pub fn open_connect_window(app: tauri::AppHandle) {
    crate::menu::open_connect_window(&app);
}

#[tauri::command]
pub async fn list_databases(
    pool: State<'_, ConnectionPool>,
    id: String,
    uri: String,
) -> Result<Vec<DatabaseInfo>, AppError> {
    let client = match pool.get_or_create(&id, &uri::with_timeout(&uri)).await {
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
pub async fn insert_document(
    pool: State<'_, ConnectionPool>,
    id: String,
    uri: String,
    database: String,
    collection: String,
    document: String,
) -> Result<String, AppError> {
    let client = match pool.get_or_create(&id, &uri::with_timeout(&uri)).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let col = client
        .database(&database)
        .collection::<bson::Document>(&collection);
    let doc = match parse_filter(&document) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let result = match col.insert_one(doc).await {
        Ok(val) => val,
        Err(e) => return Err(AppError::Mongo(e)),
    };
    Ok(result.inserted_id.to_string())
}

#[tauri::command]
pub async fn replace_document(
    pool: State<'_, ConnectionPool>,
    id: String,
    uri: String,
    database: String,
    collection: String,
    id_filter: String,
    document: String,
) -> Result<(), AppError> {
    let client = match pool.get_or_create(&id, &uri::with_timeout(&uri)).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let col = client
        .database(&database)
        .collection::<bson::Document>(&collection);
    let filter_doc = match parse_filter(&id_filter) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let mut replacement = match parse_filter(&document) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    // MongoDB errors if the replacement contains an _id that differs from the filter.
    // Remove it unconditionally — the existing _id is preserved by replace_one.
    replacement.remove("_id");
    match col.replace_one(filter_doc, replacement).await {
        Ok(_) => Ok(()),
        Err(e) => Err(AppError::Mongo(e)),
    }
}

#[tauri::command]
pub async fn delete_document(
    pool: State<'_, ConnectionPool>,
    id: String,
    uri: String,
    database: String,
    collection: String,
    id_filter: String,
) -> Result<(), AppError> {
    let client = match pool.get_or_create(&id, &uri::with_timeout(&uri)).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let col = client
        .database(&database)
        .collection::<bson::Document>(&collection);
    let filter_doc = match parse_filter(&id_filter) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    match col.delete_one(filter_doc).await {
        Ok(_) => Ok(()),
        Err(e) => Err(AppError::Mongo(e)),
    }
}
