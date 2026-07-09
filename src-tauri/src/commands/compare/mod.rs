use crate::error::AppError;
use mongodb::bson;
use serde::Serialize;
use std::collections::HashMap;
use tauri::State;

use super::{collect_documents, MAX_QUERY_TIME, AppContext};

const DEFAULT_SCAN: i64 = 2000;
const MAX_SCAN: i64 = 20_000;
const SAMPLE_CAP: usize = 100;

// One document that differs between the two collections (same _id, different content).
#[derive(Serialize)]
pub struct DiffPair {
    pub id: String,
    pub source: serde_json::Value,
    pub target: serde_json::Value,
}

#[derive(Serialize)]
pub struct DiffResult {
    pub source_total: u64,
    pub target_total: u64,
    pub only_in_source_count: u64,
    pub only_in_target_count: u64,
    pub differing_count: u64,
    pub identical_count: u64,
    // Capped samples for display.
    pub only_in_source: Vec<serde_json::Value>,
    pub only_in_target: Vec<serde_json::Value>,
    pub differing: Vec<DiffPair>,
}

// A stable string key for an _id value, distinguishing type as well as value
// (e.g. the int 1 and the string "1" don't collide).
fn id_key(doc: &bson::Document) -> String {
    match doc.get("_id") {
        Some(value) => value.to_string(),
        None => String::new(),
    }
}

// Pure diff of two document sets, keyed by _id. Content equality is
// order-insensitive (compared as JSON), so a differing field-order alone does not
// count as a difference. Documents with no _id are ignored (GridFS/system aside,
// every stored document has one).
pub(crate) fn diff_docs(source: &[bson::Document], target: &[bson::Document]) -> DiffResult {
    let mut source_map: HashMap<String, &bson::Document> = HashMap::new();
    for doc in source {
        let key = id_key(doc);
        if !key.is_empty() {
            source_map.insert(key, doc);
        }
    }
    let mut target_map: HashMap<String, &bson::Document> = HashMap::new();
    for doc in target {
        let key = id_key(doc);
        if !key.is_empty() {
            target_map.insert(key, doc);
        }
    }

    let mut result = DiffResult {
        source_total: source_map.len() as u64,
        target_total: target_map.len() as u64,
        only_in_source_count: 0,
        only_in_target_count: 0,
        differing_count: 0,
        identical_count: 0,
        only_in_source: Vec::new(),
        only_in_target: Vec::new(),
        differing: Vec::new(),
    };

    // Walk source: classify as only-in-source, identical, or differing.
    for (key, src_doc) in &source_map {
        match target_map.get(key) {
            None => {
                result.only_in_source_count += 1;
                if result.only_in_source.len() < SAMPLE_CAP {
                    result.only_in_source.push(serde_json::Value::from(bson::Bson::Document((*src_doc).clone())));
                }
            }
            Some(tgt_doc) => {
                let src_json = serde_json::Value::from(bson::Bson::Document((*src_doc).clone()));
                let tgt_json = serde_json::Value::from(bson::Bson::Document((*tgt_doc).clone()));
                if src_json == tgt_json {
                    result.identical_count += 1;
                } else {
                    result.differing_count += 1;
                    if result.differing.len() < SAMPLE_CAP {
                        result.differing.push(DiffPair {
                            id: key.clone(),
                            source: src_json,
                            target: tgt_json,
                        });
                    }
                }
            }
        }
    }
    // Walk target for only-in-target.
    for (key, tgt_doc) in &target_map {
        if !source_map.contains_key(key) {
            result.only_in_target_count += 1;
            if result.only_in_target.len() < SAMPLE_CAP {
                result.only_in_target.push(serde_json::Value::from(bson::Bson::Document((*tgt_doc).clone())));
            }
        }
    }

    result
}

async fn load_docs(
    client: &mongodb::Client,
    database: &str,
    collection: &str,
    scan: i64,
) -> Result<Vec<bson::Document>, AppError> {
    let col = client.database(database).collection::<bson::Document>(collection);
    let mut cursor = match col.find(bson::doc! {}).limit(scan).max_time(MAX_QUERY_TIME).await {
        Ok(val) => val,
        Err(e) => return Err(AppError::Mongo(e)),
    };
    collect_documents(&mut cursor).await
}

/// Compare two collections in a database by `_id`, reporting documents only in the
/// source, only in the target, and those present in both but differing (plus an
/// identical count), the way Studio-3T's Data Compare does. Each side is scanned
/// up to `scan_limit` documents; content equality is order-insensitive.
#[tauri::command]
pub async fn compare_collections(
    ctx: State<'_, AppContext>,
    id: String,
    database: String,
    source: String,
    target: String,
    scan_limit: Option<i64>,
) -> Result<DiffResult, AppError> {
    let client = match ctx.client(&id).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let scan = match scan_limit {
        Some(val) if val > 0 => val.min(MAX_SCAN),
        _ => DEFAULT_SCAN,
    };
    let source_docs = match load_docs(&client, &database, &source, scan).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let target_docs = match load_docs(&client, &database, &target, scan).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    Ok(diff_docs(&source_docs, &target_docs))
}

#[cfg(test)]
#[path = "compare.test.rs"]
mod tests;
