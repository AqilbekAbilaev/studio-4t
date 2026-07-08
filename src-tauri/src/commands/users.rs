use crate::error::AppError;
use mongodb::bson;
use serde::Serialize;
use tauri::State;

use super::AppContext;

// A database user, normalized from the `usersInfo` result.
#[derive(Serialize)]
pub struct UserInfo {
    pub user: String,
    pub db: String,
    pub roles: Vec<String>,   // each as "role@db"
}

// Extract the role list from a user document's `roles` array into "role@db" strings.
fn extract_roles(user: &bson::Document) -> Vec<String> {
    let mut roles: Vec<String> = Vec::new();
    if let Some(bson::Bson::Array(entries)) = user.get("roles") {
        for entry in entries {
            if let bson::Bson::Document(role_doc) = entry {
                let role = match role_doc.get("role") {
                    Some(bson::Bson::String(text)) => text.clone(),
                    _ => continue,
                };
                let db = match role_doc.get("db") {
                    Some(bson::Bson::String(text)) => text.clone(),
                    _ => String::new(),
                };
                roles.push(format!("{role}@{db}"));
            }
        }
    }
    roles
}

/// List the users defined on a database (via `usersInfo`).
#[tauri::command]
pub async fn list_users(
    ctx: State<'_, AppContext>,
    id: String,
    database: String,
) -> Result<Vec<UserInfo>, AppError> {
    let client = match ctx.client(&id).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let result = match client.database(&database).run_command(bson::doc! { "usersInfo": 1 }).await {
        Ok(val) => val,
        Err(e) => return Err(AppError::Mongo(e)),
    };
    let mut users: Vec<UserInfo> = Vec::new();
    if let Some(bson::Bson::Array(entries)) = result.get("users") {
        for entry in entries {
            if let bson::Bson::Document(user_doc) = entry {
                let user = match user_doc.get("user") {
                    Some(bson::Bson::String(text)) => text.clone(),
                    _ => continue,
                };
                let db = match user_doc.get("db") {
                    Some(bson::Bson::String(text)) => text.clone(),
                    _ => database.clone(),
                };
                users.push(UserInfo {
                    user: user,
                    db: db,
                    roles: extract_roles(user_doc),
                });
            }
        }
    }
    Ok(users)
}

/// Create a database user with a password and roles. Each role is "role" (granted on
/// `database`) or "role@otherdb".
#[tauri::command]
pub async fn create_user(
    ctx: State<'_, AppContext>,
    id: String,
    database: String,
    username: String,
    password: String,
    roles: Vec<String>,
) -> Result<(), AppError> {
    let client = match ctx.client_for_write(&id).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let mut role_docs: Vec<bson::Bson> = Vec::new();
    for role in roles.iter() {
        let trimmed = role.trim();
        if trimmed.is_empty() {
            continue;
        }
        let role_doc = match trimmed.split_once('@') {
            Some((name, db)) => bson::doc! { "role": name, "db": db },
            None => bson::doc! { "role": trimmed, "db": &database },
        };
        role_docs.push(bson::Bson::Document(role_doc));
    }
    let command = bson::doc! {
        "createUser": &username,
        "pwd": &password,
        "roles": role_docs,
    };
    match client.database(&database).run_command(command).await {
        Ok(_) => Ok(()),
        Err(e) => Err(AppError::Mongo(e)),
    }
}

/// Drop a database user by name.
#[tauri::command]
pub async fn drop_user(
    ctx: State<'_, AppContext>,
    id: String,
    database: String,
    username: String,
) -> Result<(), AppError> {
    let client = match ctx.client_for_write(&id).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    match client.database(&database).run_command(bson::doc! { "dropUser": &username }).await {
        Ok(_) => Ok(()),
        Err(e) => Err(AppError::Mongo(e)),
    }
}

/// List the role names defined on a database (via `rolesInfo`; built-in roles excluded).
#[tauri::command]
pub async fn list_roles(
    ctx: State<'_, AppContext>,
    id: String,
    database: String,
) -> Result<Vec<String>, AppError> {
    let client = match ctx.client(&id).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let command = bson::doc! { "rolesInfo": 1, "showBuiltinRoles": false };
    let result = match client.database(&database).run_command(command).await {
        Ok(val) => val,
        Err(e) => return Err(AppError::Mongo(e)),
    };
    let mut roles: Vec<String> = Vec::new();
    if let Some(bson::Bson::Array(entries)) = result.get("roles") {
        for entry in entries {
            if let bson::Bson::Document(role_doc) = entry {
                if let Some(bson::Bson::String(name)) = role_doc.get("role") {
                    roles.push(name.clone());
                }
            }
        }
    }
    Ok(roles)
}
