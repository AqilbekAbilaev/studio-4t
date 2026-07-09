use crate::error::AppError;
use crate::history::{now_ms, HistoryStorage, QueryHistoryEntry};
use crate::default_queries::{DefaultQuery, DefaultQueryStorage};
use crate::node_tags::NodeTagStorage;
use crate::saved_queries::{SavedQueryEntry, SavedQueryStorage};
use crate::tabs::TabStorage;
use crate::settings::{Settings, SettingsStorage};
use std::collections::HashMap;
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub fn get_settings(settings: State<'_, SettingsStorage>) -> Settings {
    settings.load()
}

/// Persist app preferences. The default query limit is clamped to a sane range
/// so a bad value can't break paging. Returns the saved settings.
#[tauri::command]
pub fn update_settings(
    settings: State<'_, SettingsStorage>,
    default_query_limit: i64,
    theme: String,
) -> Result<Settings, AppError> {
    // Only known theme names are accepted; anything else falls back to dark so a
    // bad value from the frontend can't leave the UI in an undefined state.
    let validated_theme = match theme.as_str() {
        "light" => "light".to_string(),
        _ => "dark".to_string(),
    };
    let new_settings = Settings {
        default_query_limit: default_query_limit.clamp(1, 1000),
        theme: validated_theme,
    };
    match settings.save(&new_settings) {
        Ok(_) => Ok(new_settings),
        Err(e) => return Err(e),
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
pub fn get_open_tabs(ts: State<'_, TabStorage>) -> Option<serde_json::Value> {
    ts.load()
}

#[tauri::command]
pub fn set_open_tabs(
    ts:      State<'_, TabStorage>,
    session: serde_json::Value,
) -> Result<(), AppError> {
    match ts.save(&session) {
        Ok(val) => Ok(val),
        Err(e)  => Err(e),
    }
}

/// All persisted database/collection colour tags, as a map of node key
/// ("connId/db" or "connId/db/coll") to colour name. Loaded on startup so tags
/// survive a restart. Connection-level tags are not here — they live on the
/// connection config and come back with `list_connections`.
#[tauri::command]
pub fn get_node_tags(tags: State<'_, NodeTagStorage>) -> HashMap<String, String> {
    tags.load()
}

/// Set or clear the colour tag on a database/collection tree node. The colour
/// "none" clears the tag (removes the entry) rather than storing it.
#[tauri::command]
pub fn set_node_tag(
    tags:  State<'_, NodeTagStorage>,
    key:   String,
    color: String,
) -> Result<(), AppError> {
    let result = if color == "none" {
        tags.clear(&key)
    } else {
        tags.set(&key, &color)
    };
    match result {
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
