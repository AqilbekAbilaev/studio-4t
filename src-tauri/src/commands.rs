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
}

#[tauri::command]
pub async fn test_connection(uri: String) -> Result<(), AppError> {
    uri::tcp_probe(&uri).await?;
    let client = Client::with_uri_str(&uri::with_timeout(&uri)).await?;
    client.list_database_names().await?;
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
    storage.add(ConnectionConfig {
        id: id.clone(),
        name,
        uri: uri.clone(),
    })?;

    // Create and cache the client immediately so the first expand is instant.
    let client = Client::with_uri_str(&uri::with_timeout(&uri)).await?;
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
    storage.remove(&id)?;
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
    let json: serde_json::Value = serde_json::from_str(trimmed)?;
    bson::to_document(&json).map_err(|e| AppError::Bson(e.to_string()))
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
    let client = pool.get_or_create(&id, &uri::with_timeout(&uri)).await?;
    let col = client
        .database(&database)
        .collection::<bson::Document>(&collection);

    let filter_doc = parse_filter(&filter)?;
    let projection_doc = parse_filter(&projection)?;
    let sort_doc = parse_filter(&sort)?;

    let mut query = col.find(filter_doc).limit(limit).skip(skip as u64);
    if !projection_doc.is_empty() {
        query = query.projection(projection_doc);
    }
    if !sort_doc.is_empty() {
        query = query.sort(sort_doc);
    }

    let mut cursor = query.await?;
    let mut docs = Vec::new();
    while cursor.advance().await? {
        let doc: bson::Document = cursor.deserialize_current()?;
        // Use bson's own From impl (not serde_json::to_value) — bson's Serialize
        // targets the bson wire format, not JSON, so to_value produces wrong output.
        docs.push(serde_json::Value::from(bson::Bson::Document(doc)));
    }
    Ok(docs)
}

#[tauri::command]
pub async fn list_databases(
    pool: State<'_, ConnectionPool>,
    id: String,
    uri: String,
) -> Result<Vec<DatabaseInfo>, AppError> {
    let client = pool.get_or_create(&id, &uri::with_timeout(&uri)).await?;

    let db_names = client.list_database_names().await?;
    let mut databases = Vec::new();
    for name in db_names {
        let collections = client.database(&name).list_collection_names().await?;
        databases.push(DatabaseInfo { name, collections });
    }
    Ok(databases)
}
