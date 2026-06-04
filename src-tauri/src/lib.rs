use mongodb::Client;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::Manager;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone)]
struct ConnectionConfig {
    id: String,
    name: String,
    uri: String,
}

#[derive(Serialize, Deserialize)]
struct DatabaseInfo {
    name: String,
    collections: Vec<String>,
}

fn connections_path(app_handle: &tauri::AppHandle) -> Result<std::path::PathBuf, String> {
    app_handle
        .path()
        .app_data_dir()
        .map(|p| p.join("connections.json"))
        .map_err(|e| e.to_string())
}

fn load_connections(path: &Path) -> Vec<ConnectionConfig> {
    if !path.exists() {
        return vec![];
    }
    let content = std::fs::read_to_string(path).unwrap_or_default();
    serde_json::from_str(&content).unwrap_or_default()
}

fn save_connections(path: &Path, list: &Vec<ConnectionConfig>) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(list).map_err(|e| e.to_string())?;
    std::fs::write(path, content).map_err(|e| e.to_string())
}

#[tauri::command]
async fn test_connection(uri: String) -> Result<(), String> {
    // Fast path: async TCP probe before invoking the MongoDB driver.
    // When a port is closed the OS returns ECONNREFUSED in milliseconds,
    // so we surface that immediately instead of waiting for the driver timeout.
    // Only applies to standard mongodb:// URIs (not SRV or other schemes).
    if let Some(host_port) = extract_host_port(&uri) {
        let addrs: Vec<_> = tokio::net::lookup_host(&host_port)
            .await
            .map_err(|e| format!("Invalid address \"{}\": {}", host_port, e))?
            .collect();

        if let Some(addr) = addrs.first() {
            tokio::time::timeout(
                std::time::Duration::from_secs(3),
                tokio::net::TcpStream::connect(addr),
            )
            .await
            .map_err(|_| format!("Cannot reach {} (timed out)", host_port))?
            .map_err(|e| format!("Cannot reach {}: {}", host_port, e))?;
        }
    }

    // TCP layer is open — hand off to the MongoDB driver for auth/handshake.
    let timeout_uri = append_timeout_params(&uri);
    let client = Client::with_uri_str(&timeout_uri).await.map_err(|e| e.to_string())?;
    client
        .list_database_names()
        .await
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn list_databases(uri: String) -> Result<Vec<DatabaseInfo>, String> {
    let timeout_uri = append_timeout_params(&uri);
    let client = Client::with_uri_str(&timeout_uri).await.map_err(|e| e.to_string())?;
    let db_names = client
        .list_database_names()
        .await
        .map_err(|e| e.to_string())?;
    let mut databases = Vec::new();
    for name in db_names {
        let collections = client
            .database(&name)
            .list_collection_names()
            .await
            .map_err(|e| e.to_string())?;
        databases.push(DatabaseInfo { name, collections });
    }
    Ok(databases)
}

// Extracts "host:port" from a mongodb:// URI for the TCP probe.
// Returns None for mongodb+srv:// or anything we can't parse safely.
fn extract_host_port(uri: &str) -> Option<String> {
    let rest = uri.strip_prefix("mongodb://")?;
    // Drop credentials (user:pass@host → host)
    let rest = rest.find('@').map_or(rest, |i| &rest[i + 1..]);
    // Take the first host:port before any '/' or '?'
    let host_port = rest.split(|c| c == '/' || c == '?').next()?;
    if host_port.is_empty() {
        return None;
    }
    if host_port.contains(':') {
        Some(host_port.to_string())
    } else {
        Some(format!("{}:27017", host_port))
    }
}

// MongoDB URIs require a '/' before '?' when there is no database path.
// e.g. mongodb://localhost:27017/?timeout=5000  (valid)
//      mongodb://localhost:27017?timeout=5000   (invalid — missing slash)
fn append_timeout_params(uri: &str) -> String {
    let params = "serverSelectionTimeoutMS=5000&connectTimeoutMS=5000";
    if uri.contains('?') {
        format!("{}&{}", uri, params)
    } else {
        let scheme_end = uri.find("://").map(|i| i + 3).unwrap_or(0);
        let has_path_slash = uri[scheme_end..].contains('/');
        if has_path_slash {
            format!("{}?{}", uri, params)
        } else {
            format!("{}/?{}", uri, params)
        }
    }
}

#[tauri::command]
fn save_connection(
    app_handle: tauri::AppHandle,
    name: String,
    uri: String,
) -> Result<String, String> {
    let path = connections_path(&app_handle)?;
    let mut connections = load_connections(&path);
    let id = Uuid::new_v4().to_string();
    connections.push(ConnectionConfig {
        id: id.clone(),
        name,
        uri,
    });
    save_connections(&path, &connections)?;
    Ok(id)
}

#[tauri::command]
fn list_connections(app_handle: tauri::AppHandle) -> Result<Vec<ConnectionConfig>, String> {
    let path = connections_path(&app_handle)?;
    Ok(load_connections(&path))
}

#[tauri::command]
fn delete_connection(app_handle: tauri::AppHandle, id: String) -> Result<(), String> {
    let path = connections_path(&app_handle)?;
    let mut connections = load_connections(&path);
    connections.retain(|c| c.id != id);
    save_connections(&path, &connections)
}

#[tauri::command]
fn greet(name: &str, _age: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            test_connection,
            save_connection,
            list_connections,
            delete_connection,
            list_databases,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
