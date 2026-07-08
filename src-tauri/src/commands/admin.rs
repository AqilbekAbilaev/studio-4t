use crate::error::AppError;
use mongodb::bson;
use mongodb::options::IndexOptions;
use mongodb::IndexModel;
use serde::Serialize;
use tauri::State;

use super::portmap::{apply_field_map, FieldMap};
use super::{
    collect_values, parse_ejson_document, parse_pipeline, DatabaseInfo, AppContext,
};

/// The `_id_` index is created and required by MongoDB and can never be dropped,
/// hidden, or otherwise modified. The index-management guards share this check so
/// the rule lives in one place (kept pure so it can be unit-tested).
pub fn is_protected_index(name: &str) -> bool {
    name == "_id_"
}

#[tauri::command]
pub async fn list_databases(
    ctx: State<'_, AppContext>,
    id: String,
) -> Result<Vec<DatabaseInfo>, AppError> {
    let client = match ctx.client(&id).await {
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
    ctx: State<'_, AppContext>,
    id: String,
    database: String,
    name: String,
) -> Result<(), AppError> {
    let client = match ctx.client(&id).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    match client.database(&database).create_collection(&name).await {
        Ok(val) => Ok(val),
        Err(e) => Err(AppError::Mongo(e)),
    }
}

/// Create a view: a read-only collection defined by an aggregation `pipeline` over a
/// source collection (`view_on`). Uses the `create` command with viewOn + pipeline.
#[tauri::command]
pub async fn create_view(
    ctx: State<'_, AppContext>,
    id: String,
    database: String,
    name: String,
    view_on: String,
    pipeline: String,
) -> Result<(), AppError> {
    // Parse and validate the pipeline before opening a connection, so a bad pipeline
    // fails fast with a clear message rather than a driver error.
    let stages = match parse_pipeline(&pipeline) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let client = match ctx.client(&id).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let pipeline_bson: Vec<bson::Bson> = stages.into_iter().map(bson::Bson::Document).collect();
    let command = bson::doc! {
        "create": &name,
        "viewOn": &view_on,
        "pipeline": pipeline_bson,
    };
    match client.database(&database).run_command(command).await {
        Ok(_) => Ok(()),
        Err(e) => Err(AppError::Mongo(e)),
    }
}

// The current schema-validation settings for a collection, read from its options so
// the editor can prefill rather than silently overwrite an existing rule.
#[derive(Serialize)]
pub struct ValidatorInfo {
    pub validator: Option<String>,          // the validator document as pretty JSON, if any
    pub validation_level: Option<String>,   // "off" | "moderate" | "strict"
    pub validation_action: Option<String>,  // "error" | "warn"
}

/// Read a collection's current schema validator (via `listCollections`) so the
/// validator editor can prefill. Returns empty fields when no validator is set.
#[tauri::command]
pub async fn get_validator(
    ctx: State<'_, AppContext>,
    id: String,
    database: String,
    collection: String,
) -> Result<ValidatorInfo, AppError> {
    let client = match ctx.client(&id).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let command = bson::doc! { "listCollections": 1, "filter": { "name": &collection } };
    let result = match client.database(&database).run_command(command).await {
        Ok(val) => val,
        Err(e) => return Err(AppError::Mongo(e)),
    };
    let empty = ValidatorInfo {
        validator: None,
        validation_level: None,
        validation_action: None,
    };
    // Navigate result.cursor.firstBatch[0].options, tolerating any missing level.
    let cursor = match result.get("cursor") {
        Some(bson::Bson::Document(doc)) => doc,
        _ => return Ok(empty),
    };
    let first_batch = match cursor.get("firstBatch") {
        Some(bson::Bson::Array(arr)) => arr,
        _ => return Ok(empty),
    };
    let entry = match first_batch.first() {
        Some(bson::Bson::Document(doc)) => doc,
        _ => return Ok(empty),
    };
    let options = match entry.get("options") {
        Some(bson::Bson::Document(doc)) => doc,
        _ => return Ok(empty),
    };
    let validator = match options.get("validator") {
        Some(bson::Bson::Document(doc)) if !doc.is_empty() => {
            let value = serde_json::Value::from(bson::Bson::Document(doc.clone()));
            match serde_json::to_string_pretty(&value) {
                Ok(text) => Some(text),
                Err(_) => None,
            }
        }
        _ => None,
    };
    let validation_level = match options.get("validationLevel") {
        Some(bson::Bson::String(text)) => Some(text.clone()),
        _ => None,
    };
    let validation_action = match options.get("validationAction") {
        Some(bson::Bson::String(text)) => Some(text.clone()),
        _ => None,
    };
    Ok(ValidatorInfo {
        validator: validator,
        validation_level: validation_level,
        validation_action: validation_action,
    })
}

/// Set (or clear) a collection's schema validator via `collMod`. An empty validator
/// clears the rule. `validation_level`/`validation_action` are passed through.
#[tauri::command]
pub async fn set_validator(
    ctx: State<'_, AppContext>,
    id: String,
    database: String,
    collection: String,
    validator: String,
    validation_level: String,
    validation_action: String,
) -> Result<(), AppError> {
    // Parse the validator up front; an empty string clears the rule (validator: {}).
    let validator_doc = if validator.trim().is_empty() || validator.trim() == "{}" {
        bson::Document::new()
    } else {
        match parse_ejson_document(&validator) {
            Ok(val) => val,
            Err(e) => return Err(e),
        }
    };
    let client = match ctx.client(&id).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let command = bson::doc! {
        "collMod": &collection,
        "validator": validator_doc,
        "validationLevel": &validation_level,
        "validationAction": &validation_action,
    };
    match client.database(&database).run_command(command).await {
        Ok(_) => Ok(()),
        Err(e) => Err(AppError::Mongo(e)),
    }
}

#[tauri::command]
pub async fn drop_database(
    ctx: State<'_, AppContext>,
    id: String,
    database: String,
) -> Result<(), AppError> {
    let client = match ctx.client(&id).await {
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
    ctx: State<'_, AppContext>,
    id: String,
    database: String,
    collection: String,
) -> Result<(), AppError> {
    let client = match ctx.client(&id).await {
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
    ctx: State<'_, AppContext>,
    id: String,
    database: String,
    collection: String,
    new_name: String,
) -> Result<(), AppError> {
    let client = match ctx.client(&id).await {
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
    ctx: State<'_, AppContext>,
    id: String,
    database: String,
    first_collection: String,
) -> Result<(), AppError> {
    let client = match ctx.client(&id).await {
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
    ctx: State<'_, AppContext>,
    id: String,
) -> Result<serde_json::Value, AppError> {
    let client = match ctx.client(&id).await {
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

/// Run `dbStats` for a database — collection/object counts and data/storage/index
/// sizes. Returned raw as JSON; the frontend surfaces the headline fields.
#[tauri::command]
pub async fn database_stats(
    ctx: State<'_, AppContext>,
    id: String,
    database: String,
) -> Result<serde_json::Value, AppError> {
    let client = match ctx.client(&id).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let command = bson::doc! { "dbStats": 1 };
    let result = match client.database(&database).run_command(command).await {
        Ok(val) => val,
        Err(e) => return Err(AppError::Mongo(e)),
    };
    Ok(serde_json::Value::from(bson::Bson::Document(result)))
}

/// Run admin `currentOp` for a connection — the operations currently in progress on
/// the server. Returned raw as JSON; the frontend lists the `inprog` array.
#[tauri::command]
pub async fn current_ops(
    ctx: State<'_, AppContext>,
    id: String,
) -> Result<serde_json::Value, AppError> {
    let client = match ctx.client(&id).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let command = bson::doc! { "currentOp": 1 };
    let result = match client.database("admin").run_command(command).await {
        Ok(val) => val,
        Err(e) => return Err(AppError::Mongo(e)),
    };
    Ok(serde_json::Value::from(bson::Bson::Document(result)))
}

#[tauri::command]
pub async fn list_indexes(
    ctx: State<'_, AppContext>,
    id: String,
    database: String,
    collection: String,
) -> Result<Vec<serde_json::Value>, AppError> {
    let client = match ctx.client(&id).await {
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
    ctx: State<'_, AppContext>,
    id: String,
    database: String,
    collection: String,
    keys: String,
    unique: bool,
    name: String,
) -> Result<(), AppError> {
    let client = match ctx.client(&id).await {
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
    ctx: State<'_, AppContext>,
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
    let client = match ctx.client(&id).await {
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
    ctx: State<'_, AppContext>,
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
    let client = match ctx.client(&id).await {
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
    ctx: State<'_, AppContext>,
    id: String,
    database: String,
    collection: String,
) -> Result<Vec<serde_json::Value>, AppError> {
    let client = match ctx.client(&id).await {
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
    collect_values(&mut cursor).await
}

#[tauri::command]
pub async fn export_collection(
    ctx: State<'_, AppContext>,
    id: String,
    database: String,
    collection: String,
    path: String,
    format: String,
) -> Result<usize, AppError> {
    let client = match ctx.client(&id).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let col = client
        .database(&database)
        .collection::<bson::Document>(&collection);
    // Plain export: no filter, no limit, no time cap (a large export legitimately
    // takes a while), no per-document transform.
    super::stream_export(
        &col,
        bson::doc! {},
        None,
        None,
        &path,
        &format,
        |_doc: &mut bson::Document| -> Result<(), AppError> { Ok(()) },
    )
    .await
}

#[tauri::command]
pub async fn import_collection(
    ctx: State<'_, AppContext>,
    id: String,
    database: String,
    collection: String,
    path: String,
    format: String,
) -> Result<usize, AppError> {
    let client = match ctx.client(&id).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let col = client
        .database(&database)
        .collection::<bson::Document>(&collection);
    // Stream the file in bounded batches instead of loading + parsing + inserting the
    // whole thing at once, so a large import can't exhaust memory. `None` mapping =
    // insert documents exactly as parsed (also the Tasks import path).
    super::stream_import(&col, &path, &format, None).await
}

/// Field-mapping import for the Import/Export wizard: same streaming import as
/// `import_collection`, but each parsed document is rewritten through `mapping`
/// (rename source→target, coerce per-field type, drop unmapped columns) before
/// insertion. An empty `mapping` falls back to a plain import. Returns the number
/// of documents inserted.
#[tauri::command]
pub async fn import_collection_mapped(
    ctx: State<'_, AppContext>,
    id: String,
    database: String,
    collection: String,
    path: String,
    format: String,
    mapping: Vec<FieldMap>,
) -> Result<usize, AppError> {
    let client = match ctx.client(&id).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let col = client
        .database(&database)
        .collection::<bson::Document>(&collection);
    let mapping_opt = if mapping.is_empty() {
        None
    } else {
        Some(mapping)
    };
    super::stream_import(&col, &path, &format, mapping_opt).await
}

/// Field-selecting export for the Import/Export wizard: same streaming export as
/// `export_collection`, but each document is rewritten through `fields`
/// (select/reorder/rename, with optional per-field coercion) before it's written.
/// The field order drives the output column/key order. An empty `fields` exports
/// every field unchanged. Returns the number of documents written.
#[tauri::command]
pub async fn export_collection_fields(
    ctx: State<'_, AppContext>,
    id: String,
    database: String,
    collection: String,
    path: String,
    format: String,
    fields: Vec<FieldMap>,
) -> Result<usize, AppError> {
    let client = match ctx.client(&id).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let col = client
        .database(&database)
        .collection::<bson::Document>(&collection);
    super::stream_export(
        &col,
        bson::doc! {},
        None,
        None,
        &path,
        &format,
        move |doc: &mut bson::Document| -> Result<(), AppError> {
            // No fields chosen → leave the document untouched (export everything).
            if !fields.is_empty() {
                *doc = apply_field_map(doc, &fields);
            }
            Ok(())
        },
    )
    .await
}
