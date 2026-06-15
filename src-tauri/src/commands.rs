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

fn parse_filter(filter: &str) -> Result<bson::Document, AppError> {
    let trimmed = filter.trim();
    if trimmed.is_empty() || trimmed == "{}" {
        return Ok(bson::doc! {});
    }
    let json: serde_json::Value = match serde_json::from_str(trimmed) {
        Ok(val) => val,
        Err(e) => return Err(AppError::Bson(format!(
            // serde_json's "key must be a string" fires on unquoted keys like { name: 1 }.
            // The frontend preprocesses these, so if we still hit this, surface a clear hint.
            "Invalid query JSON ({e}). Keys must be quoted, e.g. {{\"name\": 1}}"
        ))),
    };
    match bson::to_document(&json) {
        Ok(val) => Ok(val),
        Err(e) => Err(AppError::Bson(e.to_string())),
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
