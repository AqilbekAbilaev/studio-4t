use crate::error::AppError;
use mongodb::bson;
use serde::Serialize;
use tauri::State;

use super::{
    collect_values, next_document, parse_ejson_document, parse_json_documents, parse_pipeline,
    MAX_QUERY_TIME, AppContext,
};

/// Fallback page size when a caller sends a non-positive `limit`. MongoDB treats
/// `limit <= 0` as "no limit", which would stream an entire collection into
/// memory; the UI always sends a positive page size, so a non-positive value is
/// out of contract and we clamp it to this bound instead of fetching everything.
const FIND_LIMIT_FALLBACK: i64 = 1000;

/// Cap on how many aggregate result documents we pull into memory and hand to the
/// UI. A pipeline with no `$limit` could otherwise stream millions of documents
/// the result grid can't render anyway. Could become a setting later; a constant
/// is enough today.
const AGG_RESULT_CAP: usize = 10_000;

/// What `run_aggregate` returns: the (possibly capped) result documents plus a
/// `truncated` flag the UI uses to warn that results were cut at `AGG_RESULT_CAP`.
#[derive(Serialize)]
pub struct AggregateResult {
    pub documents: Vec<serde_json::Value>,
    pub truncated: bool,
}

#[cfg(test)]
mod tests {
    use super::{explain_verbosity, is_operator_update};
    use mongodb::bson::doc;

    #[test]
    fn operator_form_updates_are_accepted() {
        assert!(is_operator_update(&doc! { "$set": { "a": 1 } }));
        assert!(is_operator_update(&doc! { "$set": { "a": 1 }, "$unset": { "b": "" } }));
    }

    #[test]
    fn replacement_and_empty_updates_are_rejected() {
        // A plain field is replacement-style, which update_many must not accept.
        assert!(!is_operator_update(&doc! { "a": 1 }));
        // Mixed operator + field is also invalid.
        assert!(!is_operator_update(&doc! { "$set": { "a": 1 }, "b": 2 }));
        // An empty update changes nothing and is not valid operator form.
        assert!(!is_operator_update(&doc! {}));
    }

    #[test]
    fn verbosity_defaults_to_execution_stats_when_absent() {
        assert_eq!(explain_verbosity(&None).unwrap(), "executionStats");
    }

    #[test]
    fn verbosity_accepts_the_three_valid_levels() {
        assert_eq!(
            explain_verbosity(&Some("queryPlanner".to_string())).unwrap(),
            "queryPlanner"
        );
        assert_eq!(
            explain_verbosity(&Some("executionStats".to_string())).unwrap(),
            "executionStats"
        );
        assert_eq!(
            explain_verbosity(&Some("allPlansExecution".to_string())).unwrap(),
            "allPlansExecution"
        );
    }

    #[test]
    fn verbosity_rejects_an_unknown_level() {
        assert!(explain_verbosity(&Some("bogus".to_string())).is_err());
        assert!(explain_verbosity(&Some(String::new())).is_err());
    }
}

/// Best-effort cancel of a running find/aggregate identified by its `comment`
/// tag (the run id `find_documents` / `run_aggregate` stamped on the op). Uses
/// `$currentOp` (own ops only) to find the matching opid(s) and `killOp` to stop
/// them. A user can kill their own operations without elevated privileges; on
/// locked-down deployments this may still be denied, which surfaces as an error.
/// Returns the number of operations killed (0 if it already finished).
#[tauri::command]
pub async fn kill_query(
    ctx: State<'_, AppContext>,
    id: String,
    comment: String,
) -> Result<usize, AppError> {
    let client = match ctx.client(&id).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let admin = client.database("admin");

    // $currentOp defaults to the authenticated user's own ops, which is exactly
    // the find/aggregate we tagged — and needs no inprog privilege.
    let pipeline = vec![
        bson::doc! { "$currentOp": {} },
        bson::doc! { "$match": { "command.comment": &comment } },
    ];
    let mut cursor = match admin.aggregate(pipeline).await {
        Ok(val) => val,
        Err(e) => return Err(AppError::Mongo(e)),
    };

    let mut killed = 0;
    loop {
        let op: bson::Document = match next_document(&mut cursor).await {
            Ok(Some(value)) => value,
            Ok(None) => break,
            Err(e) => return Err(e),
        };
        // `opid` is an integer on mongod or a "shard:opid" string on mongos; pass
        // whichever straight back to killOp.
        if let Some(opid) = op.get("opid") {
            let kill_command = bson::doc! { "killOp": 1, "op": opid.clone() };
            match admin.run_command(kill_command).await {
                Ok(_) => killed += 1,
                Err(e) => return Err(AppError::Mongo(e)),
            };
        }
    }
    Ok(killed)
}

#[tauri::command]
pub async fn find_documents(
    ctx: State<'_, AppContext>,
    id: String,
    database: String,
    collection: String,
    filter: String,
    projection: String,
    sort: String,
    skip: i64,
    limit: i64,
    comment: Option<String>,
) -> Result<Vec<serde_json::Value>, AppError> {
    let col = match ctx.collection(&id, &database, &collection).await {
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

    // A positive limit is self-bounding; only a non-positive one would fetch the
    // whole collection, so clamp that case to a safe page (see FIND_LIMIT_FALLBACK).
    let effective_limit = if limit <= 0 { FIND_LIMIT_FALLBACK } else { limit };
    let mut query = col
        .find(filter_doc)
        .limit(effective_limit)
        .skip(skip as u64)
        .max_time(MAX_QUERY_TIME);
    // Tag the op with the run id so kill_query can find and cancel it.
    if let Some(c) = comment.as_deref().filter(|s| !s.is_empty()) {
        query = query.comment(c.to_string());
    }
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
    collect_values(&mut cursor).await
}

/// Count the documents matching `filter` (the same filter shape `find_documents`
/// accepts). Used for the "Count Documents" action and to jump to the last page.
#[tauri::command]
pub async fn count_documents(
    ctx: State<'_, AppContext>,
    id: String,
    database: String,
    collection: String,
    filter: String,
) -> Result<i64, AppError> {
    let col = match ctx.collection(&id, &database, &collection).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };

    let filter_doc = match parse_ejson_document(&filter) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };

    let count = match col
        .count_documents(filter_doc)
        .max_time(MAX_QUERY_TIME)
        .await
    {
        Ok(val) => val,
        Err(e) => return Err(AppError::Mongo(e)),
    };
    Ok(count as i64)
}

#[tauri::command]
pub async fn insert_document(
    ctx: State<'_, AppContext>,
    id: String,
    database: String,
    collection: String,
    document: String,
) -> Result<String, AppError> {
    let col = match ctx.collection_for_write(&id, &database, &collection).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
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

// Insert one or many documents from a single Extended-JSON string — the Edit menu's
// "Paste Document(s)". The text may be a single object or a JSON array of objects;
// `parse_json_documents` validates it and surfaces a human-readable error on bad
// input (so a failed paste is a toast, not a crash). Returns the number inserted.
#[tauri::command]
pub async fn insert_documents(
    ctx: State<'_, AppContext>,
    id: String,
    database: String,
    collection: String,
    documents: String,
) -> Result<usize, AppError> {
    let docs = match parse_json_documents(&documents) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    if docs.is_empty() {
        return Err(AppError::Bson(
            "Clipboard has no document(s) to paste".to_string(),
        ));
    }
    let col = match ctx.collection_for_write(&id, &database, &collection).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    match col.insert_many(docs).await {
        Ok(result) => Ok(result.inserted_ids.len()),
        Err(e) => Err(AppError::Mongo(e)),
    }
}

#[tauri::command]
pub async fn replace_document(
    ctx: State<'_, AppContext>,
    id: String,
    database: String,
    collection: String,
    id_filter: String,
    document: String,
) -> Result<(), AppError> {
    let col = match ctx.collection_for_write(&id, &database, &collection).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
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
    ctx: State<'_, AppContext>,
    id: String,
    database: String,
    collection: String,
    id_filter: String,
) -> Result<(), AppError> {
    let col = match ctx.collection_for_write(&id, &database, &collection).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let filter_doc = match parse_ejson_document(&id_filter) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    match col.delete_one(filter_doc).await {
        Ok(_) => Ok(()),
        Err(e) => Err(AppError::Mongo(e)),
    }
}

/// Whether an update document is in operator form (every top-level key starts with
/// `$`, e.g. `{ "$set": … }`). update_many rejects replacement-style documents, so we
/// check up front to return a clear message instead of a raw driver error. An empty
/// update is not valid operator form.
fn is_operator_update(update: &bson::Document) -> bool {
    !update.is_empty() && update.keys().all(|key| key.starts_with('$'))
}

/// Update every document matching `filter` with the given `update` document (which
/// must contain update operators such as `$set` / `$unset`). Backs the Collection →
/// Update Dialog. Returns the number of documents modified.
#[tauri::command]
pub async fn update_many(
    ctx: State<'_, AppContext>,
    id: String,
    database: String,
    collection: String,
    filter: String,
    update: String,
) -> Result<i64, AppError> {
    let col = match ctx.collection_for_write(&id, &database, &collection).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let filter_doc = match parse_ejson_document(&filter) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let update_doc = match parse_ejson_document(&update) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    // Guard against a replacement-style document reaching update_many, which the
    // driver rejects: every top-level key of an update must be an operator ($set…).
    if !is_operator_update(&update_doc) {
        return Err(AppError::Bson(
            "Update must use operators, e.g. { \"$set\": { \"field\": value } }".to_string(),
        ));
    }
    let result = match col.update_many(filter_doc, update_doc).await {
        Ok(val) => val,
        Err(e) => return Err(AppError::Mongo(e)),
    };
    Ok(result.modified_count as i64)
}

/// Delete every document matching `filter`. Backs the Collection → Delete Dialog.
/// The caller is responsible for confirming the operation. Returns the number of
/// documents deleted.
#[tauri::command]
pub async fn delete_many(
    ctx: State<'_, AppContext>,
    id: String,
    database: String,
    collection: String,
    filter: String,
) -> Result<i64, AppError> {
    let col = match ctx.collection_for_write(&id, &database, &collection).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let filter_doc = match parse_ejson_document(&filter) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let result = match col.delete_many(filter_doc).await {
        Ok(val) => val,
        Err(e) => return Err(AppError::Mongo(e)),
    };
    Ok(result.deleted_count as i64)
}

/// Delete every document in the collection while keeping the (empty) collection and
/// its indexes — the "Clear Collection" action, distinct from dropping it. The caller
/// is responsible for confirming. Returns the number of documents removed.
#[tauri::command]
pub async fn clear_collection(
    ctx: State<'_, AppContext>,
    id: String,
    database: String,
    collection: String,
) -> Result<i64, AppError> {
    let col = match ctx.collection_for_write(&id, &database, &collection).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    // An empty filter matches every document; the collection itself is untouched.
    let result = match col.delete_many(bson::doc! {}).await {
        Ok(val) => val,
        Err(e) => return Err(AppError::Mongo(e)),
    };
    Ok(result.deleted_count as i64)
}

/// Resolve the requested explain verbosity to a valid MongoDB value. `None` defaults
/// to `executionStats` (the level that carries runtime numbers, which the graph draws).
/// Only the three real verbosity levels are accepted; anything else is rejected up
/// front rather than sent to the server as a bogus command. Pure, so it's unit-tested.
pub(crate) fn explain_verbosity(verbosity: &Option<String>) -> Result<String, String> {
    match verbosity {
        None => Ok("executionStats".to_string()),
        Some(value) => match value.as_str() {
            "queryPlanner" => Ok("queryPlanner".to_string()),
            "executionStats" => Ok("executionStats".to_string()),
            "allPlansExecution" => Ok("allPlansExecution".to_string()),
            other => Err(format!(
                "Invalid explain verbosity: {other} (expected queryPlanner, executionStats, or allPlansExecution)"
            )),
        },
    }
}

#[tauri::command]
pub async fn explain_query(
    ctx: State<'_, AppContext>,
    id: String,
    database: String,
    collection: String,
    filter: String,
    projection: String,
    sort: String,
    skip: i64,
    limit: i64,
    verbosity: Option<String>,
) -> Result<serde_json::Value, AppError> {
    let verbosity_value = match explain_verbosity(&verbosity) {
        Ok(val) => val,
        Err(e) => return Err(AppError::Bson(e)),
    };
    let client = match ctx.client(&id).await {
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
        "verbosity": verbosity_value,
    };
    let result = match client.database(&database).run_command(explain_command).await {
        Ok(val) => val,
        Err(e) => return Err(AppError::Mongo(e)),
    };
    Ok(serde_json::Value::from(bson::Bson::Document(result)))
}

/// Explain an aggregation pipeline: wrap the equivalent `aggregate` command in
/// `explain` and report how the server would execute it. Mirrors `explain_query`
/// but for the pipeline path (`run_aggregate`). Read path.
#[tauri::command]
pub async fn explain_aggregate(
    ctx: State<'_, AppContext>,
    id: String,
    database: String,
    collection: String,
    pipeline: String,
    verbosity: Option<String>,
) -> Result<serde_json::Value, AppError> {
    let verbosity_value = match explain_verbosity(&verbosity) {
        Ok(val) => val,
        Err(e) => return Err(AppError::Bson(e)),
    };
    let client = match ctx.client(&id).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let stages = match parse_pipeline(&pipeline) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    // The explain wrapper needs the pipeline as a BSON array of stage documents.
    let mut pipeline_bson: Vec<bson::Bson> = Vec::new();
    for stage in stages {
        pipeline_bson.push(bson::Bson::Document(stage));
    }
    let aggregate_command = bson::doc! {
        "aggregate": collection,
        "pipeline": pipeline_bson,
        "cursor": bson::doc! {},
    };
    let explain_command = bson::doc! {
        "explain": aggregate_command,
        "verbosity": verbosity_value,
    };
    let result = match client.database(&database).run_command(explain_command).await {
        Ok(val) => val,
        Err(e) => return Err(AppError::Mongo(e)),
    };
    Ok(serde_json::Value::from(bson::Bson::Document(result)))
}

#[tauri::command]
pub async fn run_aggregate(
    ctx: State<'_, AppContext>,
    id: String,
    database: String,
    collection: String,
    pipeline: String,
    comment: Option<String>,
) -> Result<AggregateResult, AppError> {
    let col = match ctx.collection(&id, &database, &collection).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let stages = match parse_pipeline(&pipeline) {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let mut aggregate = col.aggregate(stages).max_time(MAX_QUERY_TIME);
    // Tag the op with the run id so kill_query can find and cancel it.
    if let Some(c) = comment.as_deref().filter(|s| !s.is_empty()) {
        aggregate = aggregate.comment(c.to_string());
    }
    let mut cursor = match aggregate.await {
        Ok(val) => val,
        Err(e) => return Err(AppError::Mongo(e)),
    };
    let mut documents = Vec::new();
    let mut truncated = false;
    loop {
        let doc: bson::Document = match next_document(&mut cursor).await {
            Ok(Some(value)) => value,
            Ok(None) => break,
            Err(e) => return Err(e),
        };
        // We already hold CAP docs and the cursor yielded another: mark truncated
        // and stop, dropping this extra doc — bounds the result to CAP docs
        // regardless of pipeline size.
        if documents.len() >= AGG_RESULT_CAP {
            truncated = true;
            break;
        }
        documents.push(serde_json::Value::from(bson::Bson::Document(doc)));
    }
    Ok(AggregateResult { documents: documents, truncated: truncated })
}
