use crate::error::AppError;
use crate::history::{now_ms, HistoryStorage, QueryHistoryEntry};
use crate::default_queries::{DefaultQuery, DefaultQueryStorage};
use crate::saved_queries::{SavedQueryEntry, SavedQueryStorage};
use crate::pool::ConnectionPool;
use crate::storage::{ConnectionConfig, Storage};
use crate::uri;
use mongodb::bson;
use mongodb::options::IndexOptions;
use mongodb::Client;
use mongodb::IndexModel;
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
    host: String,
    port: u16,
    connection_type: String,
    replica_set_name: Option<String>,
    username: Option<String>,
    password: Option<String>,
    auth_db: Option<String>,
    auth_mechanism: Option<String>,
    tag: Option<String>,
) -> Result<String, AppError> {
    let id = Uuid::new_v4().to_string();
    let config = ConnectionConfig {
        id: id.clone(),
        name: name,
        host: host,
        port: port,
        connection_type: connection_type,
        replica_set_name: replica_set_name,
        username: username,
        auth_db: auth_db,
        auth_mechanism: auth_mechanism,
        tag: tag,
        last_accessed: None,
        // A newly saved connection is opened in the sidebar.
        open: true,
    };

    // Store password in OS keychain before persisting the rest to disk.
    let pw_ref = password.as_deref().filter(|s| !s.is_empty());
    if let Some(pw) = pw_ref {
        match crate::keychain::set(&id, pw) {
            Ok(val) => val,
            Err(e) => return Err(e),
        };
    }

    let built_uri = uri::build_uri(&config, pw_ref);
    match storage.add(config) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };

    // Create and cache the client immediately so the first expand is instant.
    let client = match Client::with_uri_str(&uri::with_timeout(&built_uri)).await {
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
pub async fn update_connection(
    storage: State<'_, Storage>,
    pool: State<'_, ConnectionPool>,
    id: String,
    name: String,
    host: String,
    port: u16,
    connection_type: String,
    replica_set_name: Option<String>,
    username: Option<String>,
    password: Option<String>,
    auth_db: Option<String>,
    auth_mechanism: Option<String>,
    tag: Option<String>,
) -> Result<(), AppError> {
    // Preserve last_accessed and the open state from the existing record.
    let existing = storage.find(&id);
    let last_accessed = existing.as_ref().and_then(|c| c.last_accessed.clone());
    let open = existing.as_ref().map(|c| c.open).unwrap_or(true);

    let config = ConnectionConfig {
        id: id.clone(),
        name: name,
        host: host,
        port: port,
        connection_type: connection_type,
        replica_set_name: replica_set_name,
        username: username,
        auth_db: auth_db,
        auth_mechanism: auth_mechanism,
        tag: tag,
        last_accessed: last_accessed,
        open: open,
    };

    // Update keychain only when a new password is supplied; empty = keep existing.
    let pw_ref = password.as_deref().filter(|s| !s.is_empty());
    if let Some(pw) = pw_ref {
        match crate::keychain::set(&id, pw) {
            Ok(val) => val,
            Err(e) => return Err(e),
        };
    }

    match storage.update(config) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };

    // Evict cached client so the next operation reconnects with updated credentials.
    pool.remove(&id).await;

    Ok(())
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
    crate::keychain::delete(&id);
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

// Decode a single Extended-JSON document into BSON. The frontend's query parser
// (utils/queryParser.js) emits canonical EJSON, so this is the decode end of that
// contract; it's used for filter / projection / sort / insert document / _id filter /
// index keys. `normalize_smart_quotes` stays as a cheap paste-safety backstop.
fn parse_ejson_document(ejson: &str) -> Result<bson::Document, AppError> {
    let trimmed = ejson.trim();
    if trimmed.is_empty() || trimmed == "{}" {
        return Ok(bson::doc! {});
    }
    let normalized = normalize_smart_quotes(trimmed);
    // Deserialize via bson::Bson so that extended-JSON types ({"$oid": "..."}, {"$date": "..."},
    // {"$numberInt": "..."}, {"$regularExpression": {...}}) decode into their BSON equivalents.
    // serde_json::Value + bson::to_document would treat {"$oid": "..."} as a plain nested
    // document, breaking _id filters.
    let bson_val: bson::Bson = match serde_json::from_str(&normalized) {
        Ok(val) => val,
        Err(e) => return Err(AppError::Bson(format!("Invalid Extended JSON ({e})"))),
    };
    match bson_val {
        bson::Bson::Document(doc) => Ok(doc),
        _ => Err(AppError::Bson("Expected a JSON object".to_string())),
    }
}

// Parse an aggregation pipeline: a JSON array of stage objects. Mirrors parse_ejson_document's
// smart-quote and extended-JSON handling so pasted shell pipelines behave the same way.
fn parse_pipeline(pipeline: &str) -> Result<Vec<bson::Document>, AppError> {
    let trimmed = pipeline.trim();
    if trimmed.is_empty() || trimmed == "[]" {
        return Ok(Vec::new());
    }
    let normalized = normalize_smart_quotes(trimmed);
    let bson_val: bson::Bson = match serde_json::from_str(&normalized) {
        Ok(val) => val,
        Err(e) => return Err(AppError::Bson(format!(
            "Invalid pipeline JSON ({e}). Keys must be quoted, e.g. [{{\"$match\": {{\"name\": 1}}}}]"
        ))),
    };
    let array = match bson_val {
        bson::Bson::Array(val) => val,
        _ => return Err(AppError::Bson("Pipeline must be a JSON array of stages".to_string())),
    };
    let mut stages = Vec::new();
    for entry in array {
        match entry {
            bson::Bson::Document(doc) => stages.push(doc),
            _ => return Err(AppError::Bson("Each pipeline stage must be a JSON object".to_string())),
        }
    }
    Ok(stages)
}

// Parse an import file's JSON into documents: either a top-level array of objects
// or a single object. Reuses the same smart-quote / extended-JSON handling as queries.
fn parse_json_documents(text: &str) -> Result<Vec<bson::Document>, AppError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Ok(Vec::new());
    }
    let normalized = normalize_smart_quotes(trimmed);
    let bson_val: bson::Bson = match serde_json::from_str(&normalized) {
        Ok(val) => val,
        Err(e) => return Err(AppError::Bson(format!(
            "Invalid JSON ({e}). Expected an array of documents."
        ))),
    };
    let array = match bson_val {
        bson::Bson::Array(val) => val,
        bson::Bson::Document(doc) => vec![bson::Bson::Document(doc)],
        _ => return Err(AppError::Bson("Import file must be a JSON array of documents".to_string())),
    };
    let mut docs = Vec::new();
    for entry in array {
        match entry {
            bson::Bson::Document(doc) => docs.push(doc),
            _ => return Err(AppError::Bson("Each item must be a JSON object".to_string())),
        }
    }
    Ok(docs)
}

// Quote a CSV field only when it contains a delimiter, quote, or newline, doubling
// any embedded quotes — standard RFC-4180 escaping.
fn csv_escape(value: &str) -> String {
    if value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r') {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_string()
    }
}

// Render a single BSON value as a flat CSV cell. Scalars become their plain text;
// anything nested (documents, arrays, dates) falls back to its JSON form.
fn bson_to_csv_cell(value: &bson::Bson) -> String {
    match value {
        bson::Bson::String(val) => val.clone(),
        bson::Bson::Boolean(val) => val.to_string(),
        bson::Bson::Int32(val) => val.to_string(),
        bson::Bson::Int64(val) => val.to_string(),
        bson::Bson::Double(val) => val.to_string(),
        bson::Bson::Null => String::new(),
        bson::Bson::ObjectId(val) => val.to_hex(),
        other => serde_json::Value::from(other.clone()).to_string(),
    }
}

fn docs_to_csv(docs: &[bson::Document]) -> String {
    // Collect the column set as the union of all keys, in first-seen order.
    let mut headers: Vec<String> = Vec::new();
    for doc in docs {
        for (key, _) in doc {
            if !headers.iter().any(|existing| existing == key) {
                headers.push(key.clone());
            }
        }
    }
    let mut out = String::new();
    let header_line: Vec<String> = headers.iter().map(|h| csv_escape(h)).collect();
    out.push_str(&header_line.join(","));
    out.push('\n');
    for doc in docs {
        let row: Vec<String> = headers
            .iter()
            .map(|header| match doc.get(header) {
                Some(value) => csv_escape(&bson_to_csv_cell(value)),
                None => String::new(),
            })
            .collect();
        out.push_str(&row.join(","));
        out.push('\n');
    }
    out
}

// Minimal RFC-4180 CSV reader: handles quoted fields, doubled quotes, and embedded
// newlines. Returns rows of string fields.
fn parse_csv_rows(text: &str) -> Vec<Vec<String>> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut record: Vec<String> = Vec::new();
    let mut field = String::new();
    let mut in_quotes = false;
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if in_quotes {
            if c == '"' {
                if i + 1 < chars.len() && chars[i + 1] == '"' {
                    field.push('"');
                    i += 1;
                } else {
                    in_quotes = false;
                }
            } else {
                field.push(c);
            }
        } else {
            match c {
                '"' => in_quotes = true,
                ',' => record.push(std::mem::take(&mut field)),
                '\n' => {
                    record.push(std::mem::take(&mut field));
                    rows.push(std::mem::take(&mut record));
                }
                '\r' => {}
                _ => field.push(c),
            }
        }
        i += 1;
    }
    if !field.is_empty() || !record.is_empty() {
        record.push(field);
        rows.push(record);
    }
    rows
}

// Best-effort type coercion for a CSV cell: empty → null, true/false → bool,
// integer/float → number, everything else → string.
fn coerce_csv_value(cell: &str) -> bson::Bson {
    let trimmed = cell.trim();
    if trimmed.is_empty() {
        return bson::Bson::Null;
    }
    if trimmed == "true" {
        return bson::Bson::Boolean(true);
    }
    if trimmed == "false" {
        return bson::Bson::Boolean(false);
    }
    match trimmed.parse::<i64>() {
        Ok(val) => return bson::Bson::Int64(val),
        Err(_) => {}
    }
    match trimmed.parse::<f64>() {
        Ok(val) => return bson::Bson::Double(val),
        Err(_) => {}
    }
    bson::Bson::String(cell.to_string())
}

fn csv_to_docs(text: &str) -> Vec<bson::Document> {
    let rows = parse_csv_rows(text);
    if rows.is_empty() {
        return Vec::new();
    }
    let headers = &rows[0];
    let mut docs = Vec::new();
    for row in rows.iter().skip(1) {
        // Skip blank trailing lines produced by a final newline.
        if row.iter().all(|cell| cell.is_empty()) {
            continue;
        }
        let mut doc = bson::Document::new();
        for (idx, header) in headers.iter().enumerate() {
            let cell = match row.get(idx) {
                Some(val) => val.as_str(),
                None => "",
            };
            doc.insert(header.clone(), coerce_csv_value(cell));
        }
        docs.push(doc);
    }
    docs
}

#[tauri::command]
pub async fn find_documents(
    pool: State<'_, ConnectionPool>,
    storage: State<'_, Storage>,
    id: String,
    database: String,
    collection: String,
    filter: String,
    projection: String,
    sort: String,
    skip: i64,
    limit: i64,
) -> Result<Vec<serde_json::Value>, AppError> {
    let config = match storage.find(&id) {
        Some(val) => val,
        None => return Err(AppError::UnknownConnection(id)),
    };
    let password = crate::keychain::get(&id);
    let built_uri = uri::build_uri(&config, password.as_deref());
    let client = match pool.get_or_create(&id, &uri::with_timeout(&built_uri)).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let col = client
        .database(&database)
        .collection::<bson::Document>(&collection);

    let filter_doc = match parse_ejson_document(&filter) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let projection_doc = match parse_ejson_document(&projection) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let sort_doc = match parse_ejson_document(&sort) {
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
pub fn set_connection_open(
    storage: State<'_, Storage>,
    id: String,
    open: bool,
) -> Result<(), AppError> {
    let mut connections = storage.load();
    if let Some(c) = connections.iter_mut().find(|c| c.id == id) {
        c.open = open;
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
    storage: State<'_, Storage>,
    id: String,
) -> Result<Vec<DatabaseInfo>, AppError> {
    let config = match storage.find(&id) {
        Some(val) => val,
        None => return Err(AppError::UnknownConnection(id)),
    };
    let password = crate::keychain::get(&id);
    let built_uri = uri::build_uri(&config, password.as_deref());
    let client = match pool.get_or_create(&id, &uri::with_timeout(&built_uri)).await {
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
    let built_uri = uri::build_uri(&config, password.as_deref());
    let client = match pool.get_or_create(&id, &uri::with_timeout(&built_uri)).await {
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
    let built_uri = uri::build_uri(&config, password.as_deref());
    let client = match pool.get_or_create(&id, &uri::with_timeout(&built_uri)).await {
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
    let built_uri = uri::build_uri(&config, password.as_deref());
    let client = match pool.get_or_create(&id, &uri::with_timeout(&built_uri)).await {
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
    let built_uri = uri::build_uri(&config, password.as_deref());
    let client = match pool.get_or_create(&id, &uri::with_timeout(&built_uri)).await {
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
    let built_uri = uri::build_uri(&config, password.as_deref());
    let client = match pool.get_or_create(&id, &uri::with_timeout(&built_uri)).await {
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

#[tauri::command]
pub async fn insert_document(
    pool: State<'_, ConnectionPool>,
    storage: State<'_, Storage>,
    id: String,
    database: String,
    collection: String,
    document: String,
) -> Result<String, AppError> {
    let config = match storage.find(&id) {
        Some(val) => val,
        None => return Err(AppError::UnknownConnection(id)),
    };
    let password = crate::keychain::get(&id);
    let built_uri = uri::build_uri(&config, password.as_deref());
    let client = match pool.get_or_create(&id, &uri::with_timeout(&built_uri)).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let col = client
        .database(&database)
        .collection::<bson::Document>(&collection);
    let doc = match parse_ejson_document(&document) {
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
    storage: State<'_, Storage>,
    id: String,
    database: String,
    collection: String,
    id_filter: String,
    document: String,
) -> Result<(), AppError> {
    let config = match storage.find(&id) {
        Some(val) => val,
        None => return Err(AppError::UnknownConnection(id)),
    };
    let password = crate::keychain::get(&id);
    let built_uri = uri::build_uri(&config, password.as_deref());
    let client = match pool.get_or_create(&id, &uri::with_timeout(&built_uri)).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let col = client
        .database(&database)
        .collection::<bson::Document>(&collection);
    let filter_doc = match parse_ejson_document(&id_filter) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let mut replacement = match parse_ejson_document(&document) {
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
    storage: State<'_, Storage>,
    id: String,
    database: String,
    collection: String,
    id_filter: String,
) -> Result<(), AppError> {
    let config = match storage.find(&id) {
        Some(val) => val,
        None => return Err(AppError::UnknownConnection(id)),
    };
    let password = crate::keychain::get(&id);
    let built_uri = uri::build_uri(&config, password.as_deref());
    let client = match pool.get_or_create(&id, &uri::with_timeout(&built_uri)).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let col = client
        .database(&database)
        .collection::<bson::Document>(&collection);
    let filter_doc = match parse_ejson_document(&id_filter) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    match col.delete_one(filter_doc).await {
        Ok(_) => Ok(()),
        Err(e) => Err(AppError::Mongo(e)),
    }
}

#[tauri::command]
pub async fn explain_query(
    pool: State<'_, ConnectionPool>,
    storage: State<'_, Storage>,
    id: String,
    database: String,
    collection: String,
    filter: String,
    projection: String,
    sort: String,
    skip: i64,
    limit: i64,
) -> Result<serde_json::Value, AppError> {
    let config = match storage.find(&id) {
        Some(val) => val,
        None => return Err(AppError::UnknownConnection(id)),
    };
    let password = crate::keychain::get(&id);
    let built_uri = uri::build_uri(&config, password.as_deref());
    let client = match pool.get_or_create(&id, &uri::with_timeout(&built_uri)).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };

    let filter_doc = match parse_ejson_document(&filter) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let projection_doc = match parse_ejson_document(&projection) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let sort_doc = match parse_ejson_document(&sort) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };

    // The `explain` command wraps the equivalent `find` command and reports how
    // the server would execute it; mirror the same optional fields find_documents uses.
    let mut find_command = bson::doc! {
        "find": collection,
        "filter": filter_doc,
    };
    if !projection_doc.is_empty() {
        find_command.insert("projection", projection_doc);
    }
    if !sort_doc.is_empty() {
        find_command.insert("sort", sort_doc);
    }
    if skip > 0 {
        find_command.insert("skip", skip);
    }
    if limit > 0 {
        find_command.insert("limit", limit);
    }

    let explain_command = bson::doc! {
        "explain": find_command,
        "verbosity": "executionStats",
    };
    let result = match client.database(&database).run_command(explain_command).await {
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
    let built_uri = uri::build_uri(&config, password.as_deref());
    let client = match pool.get_or_create(&id, &uri::with_timeout(&built_uri)).await {
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
    let built_uri = uri::build_uri(&config, password.as_deref());
    let client = match pool.get_or_create(&id, &uri::with_timeout(&built_uri)).await {
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
    let config = match storage.find(&id) {
        Some(val) => val,
        None => return Err(AppError::UnknownConnection(id)),
    };
    let password = crate::keychain::get(&id);
    let built_uri = uri::build_uri(&config, password.as_deref());
    let client = match pool.get_or_create(&id, &uri::with_timeout(&built_uri)).await {
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

#[tauri::command]
pub async fn run_aggregate(
    pool: State<'_, ConnectionPool>,
    storage: State<'_, Storage>,
    id: String,
    database: String,
    collection: String,
    pipeline: String,
) -> Result<Vec<serde_json::Value>, AppError> {
    let config = match storage.find(&id) {
        Some(val) => val,
        None => return Err(AppError::UnknownConnection(id)),
    };
    let password = crate::keychain::get(&id);
    let built_uri = uri::build_uri(&config, password.as_deref());
    let client = match pool.get_or_create(&id, &uri::with_timeout(&built_uri)).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let col = client
        .database(&database)
        .collection::<bson::Document>(&collection);
    let stages = match parse_pipeline(&pipeline) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let mut cursor = match col.aggregate(stages).await {
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
        docs.push(serde_json::Value::from(bson::Bson::Document(doc)));
    }
    Ok(docs)
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
    let built_uri = uri::build_uri(&config, password.as_deref());
    let client = match pool.get_or_create(&id, &uri::with_timeout(&built_uri)).await {
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
    let built_uri = uri::build_uri(&config, password.as_deref());
    let client = match pool.get_or_create(&id, &uri::with_timeout(&built_uri)).await {
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


#[tauri::command]
pub fn get_default_query(
    dq:            State<'_, DefaultQueryStorage>,
    connection_id: String,
    database:      String,
    collection:    String,
) -> Option<DefaultQuery> {
    let key = format!("{}::{}::{}", connection_id, database, collection);
    dq.get(&key)
}

#[tauri::command]
pub fn set_default_query(
    dq:            State<'_, DefaultQueryStorage>,
    connection_id: String,
    database:      String,
    collection:    String,
    mode:          String,
    filter:        String,
    sort:          String,
    projection:    String,
    skip:          i64,
    limit:         i64,
    pipeline:      String,
) -> Result<(), AppError> {
    let key = format!("{}::{}::{}", connection_id, database, collection);
    let entry = DefaultQuery {
        mode:       mode,
        filter:     filter,
        sort:       sort,
        projection: projection,
        skip:       skip,
        limit:      limit,
        pipeline:   pipeline,
    };
    match dq.set(&key, entry) {
        Ok(val) => Ok(val),
        Err(e)  => Err(e),
    }
}

#[tauri::command]
pub fn clear_default_query(
    dq:            State<'_, DefaultQueryStorage>,
    connection_id: String,
    database:      String,
    collection:    String,
) -> Result<(), AppError> {
    let key = format!("{}::{}::{}", connection_id, database, collection);
    match dq.clear(&key) {
        Ok(val) => Ok(val),
        Err(e)  => Err(e),
    }
}

#[tauri::command]
pub fn list_saved_queries(sq: State<'_, SavedQueryStorage>) -> Vec<SavedQueryEntry> {
    sq.load()
}

#[tauri::command]
pub fn save_query(
    sq:         State<'_, SavedQueryStorage>,
    name:       String,
    mode:       String,
    filter:     String,
    sort:       String,
    projection: String,
    skip:       i64,
    limit:      i64,
    pipeline:   String,
) -> Result<String, AppError> {
    let id = Uuid::new_v4().to_string();
    let entry = SavedQueryEntry {
        id:         id.clone(),
        name:       name,
        mode:       mode,
        filter:     filter,
        sort:       sort,
        projection: projection,
        skip:       skip,
        limit:      limit,
        pipeline:   pipeline,
        saved_at:   now_ms(),
    };
    match sq.insert(entry) {
        Ok(_)  => Ok(id),
        Err(e) => Err(e),
    }
}

#[tauri::command]
pub fn delete_saved_query(sq: State<'_, SavedQueryStorage>, id: String) -> Result<(), AppError> {
    match sq.delete(&id) {
        Ok(val) => Ok(val),
        Err(e)  => Err(e),
    }
}

#[tauri::command]
pub fn get_query_history(
    history: State<'_, HistoryStorage>,
    connection_id: String,
    database: String,
    collection: String,
) -> Vec<QueryHistoryEntry> {
    let key = format!("{}::{}::{}", connection_id, database, collection);
    history.get(&key)
}

#[tauri::command]
pub fn push_query_history(
    history: State<'_, HistoryStorage>,
    connection_id: String,
    database: String,
    collection: String,
    mode: String,
    filter: String,
    sort: String,
    projection: String,
    skip: i64,
    limit: i64,
    pipeline: String,
) -> Result<(), AppError> {
    let key = format!("{}::{}::{}", connection_id, database, collection);
    let entry = QueryHistoryEntry {
        id: Uuid::new_v4().to_string(),
        mode: mode,
        filter: filter,
        sort: sort,
        projection: projection,
        skip: skip,
        limit: limit,
        pipeline: pipeline,
        ran_at: now_ms(),
    };
    match history.push(&key, entry) {
        Ok(val) => Ok(val),
        Err(e) => Err(e),
    }
}

#[tauri::command]
pub fn clear_query_history(
    history: State<'_, HistoryStorage>,
    connection_id: String,
    database: String,
    collection: String,
) -> Result<(), AppError> {
    let key = format!("{}::{}::{}", connection_id, database, collection);
    match history.clear(&key) {
        Ok(val) => Ok(val),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The frontend's query parser emits canonical Extended JSON; these confirm the Rust
    // decode (the same `serde_json::from_str::<bson::Bson>` the commands use) yields the
    // right BSON types for the tricky cases.

    #[test]
    fn ejson_objectid_decodes_to_object_id() {
        let doc = parse_ejson_document(r#"{"_id":{"$oid":"507f1f77bcf86cd799439011"}}"#).unwrap();
        assert!(matches!(doc.get("_id"), Some(bson::Bson::ObjectId(_))));
    }

    #[test]
    fn ejson_date_decodes_to_datetime() {
        let doc =
            parse_ejson_document(r#"{"created":{"$date":{"$numberLong":"1704067200000"}}}"#).unwrap();
        assert!(matches!(doc.get("created"), Some(bson::Bson::DateTime(_))));
    }

    #[test]
    fn ejson_regex_decodes_to_regular_expression() {
        let doc =
            parse_ejson_document(r#"{"name":{"$regularExpression":{"pattern":"^jo","options":"i"}}}"#)
                .unwrap();
        assert!(matches!(doc.get("name"), Some(bson::Bson::RegularExpression(_))));
    }

    #[test]
    fn ejson_nested_operator_number_decodes_to_int32() {
        let doc = parse_ejson_document(r#"{"age":{"$gt":{"$numberInt":"18"}}}"#).unwrap();
        let inner = doc.get_document("age").unwrap();
        assert!(matches!(inner.get("$gt"), Some(bson::Bson::Int32(18))));
    }

    #[test]
    fn ejson_string_with_punctuation_is_preserved() {
        let doc = parse_ejson_document(r#"{"note":"hello, world: x"}"#).unwrap();
        match doc.get("note") {
            Some(bson::Bson::String(value)) => assert_eq!(value, "hello, world: x"),
            other => panic!("expected a string, got {:?}", other),
        }
    }

    #[test]
    fn empty_and_braces_decode_to_empty_document() {
        assert!(parse_ejson_document("").unwrap().is_empty());
        assert!(parse_ejson_document("{}").unwrap().is_empty());
    }

    #[test]
    fn non_object_ejson_is_an_error() {
        assert!(parse_ejson_document("[1, 2, 3]").is_err());
    }

    #[test]
    fn smart_quotes_are_normalized_as_a_backstop() {
        let doc = parse_ejson_document("{\u{201C}name\u{201D}:\u{201C}John\u{201D}}").unwrap();
        match doc.get("name") {
            Some(bson::Bson::String(value)) => assert_eq!(value, "John"),
            other => panic!("expected a string, got {:?}", other),
        }
    }

    #[test]
    fn pipeline_canonical_ejson_decodes_to_stage_documents() {
        let stages = parse_pipeline(
            r#"[{"$match":{"x":{"$numberInt":"1"}}},{"$group":{"_id":"$y","n":{"$sum":{"$numberInt":"1"}}}}]"#,
        )
        .unwrap();
        assert_eq!(stages.len(), 2);
        assert!(stages[0].contains_key("$match"));
    }

    #[test]
    fn empty_pipeline_decodes_to_no_stages() {
        assert!(parse_pipeline("[]").unwrap().is_empty());
        assert!(parse_pipeline("").unwrap().is_empty());
    }

    #[test]
    fn sort_document_preserves_key_order() {
        // The link the plan was least sure of: JS EJSON.stringify keeps key order, and
        // bson::Document must keep it through serde_json -> BSON so sort fields apply in
        // the order the user wrote them.
        let doc = parse_ejson_document(r#"{"a":{"$numberInt":"1"},"b":{"$numberInt":"-1"}}"#).unwrap();
        let keys: Vec<&str> = doc.keys().map(|k| k.as_str()).collect();
        assert_eq!(keys, vec!["a", "b"]);

        let reversed = parse_ejson_document(r#"{"b":{"$numberInt":"1"},"a":{"$numberInt":"1"}}"#).unwrap();
        let reversed_keys: Vec<&str> = reversed.keys().map(|k| k.as_str()).collect();
        assert_eq!(reversed_keys, vec!["b", "a"]);
    }
}
