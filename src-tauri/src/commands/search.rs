use crate::error::AppError;
use mongodb::bson;
use serde::Serialize;
use tauri::State;

use super::{next_document, MAX_QUERY_TIME, AppContext};

const DEFAULT_SCAN: i64 = 1000;
const MAX_SCAN: i64 = 5000;
const DEFAULT_MAX_HITS: i64 = 50;

#[derive(Serialize)]
pub struct CollectionHits {
    pub collection: String,
    pub scanned: u64,
    pub matched: u64,
    // Up to `max_hits` matching documents, as JSON for display.
    pub hits: Vec<serde_json::Value>,
}

// Does a single value contain the (already-lowercased) needle anywhere? Scalars
// are compared by their text form; documents and arrays are searched recursively.
fn value_matches(value: &bson::Bson, needle: &str) -> bool {
    match value {
        bson::Bson::String(val) => val.to_lowercase().contains(needle),
        bson::Bson::Int32(val) => val.to_string().contains(needle),
        bson::Bson::Int64(val) => val.to_string().contains(needle),
        bson::Bson::Double(val) => val.to_string().contains(needle),
        bson::Bson::Boolean(val) => val.to_string().contains(needle),
        bson::Bson::ObjectId(val) => val.to_hex().contains(needle),
        bson::Bson::Decimal128(val) => val.to_string().to_lowercase().contains(needle),
        bson::Bson::DateTime(val) => match val.try_to_rfc3339_string() {
            Ok(text) => text.to_lowercase().contains(needle),
            Err(_) => false,
        },
        bson::Bson::Document(doc) => doc_matches(doc, needle),
        bson::Bson::Array(items) => items.iter().any(|item| value_matches(item, needle)),
        _ => false,
    }
}

// True if any field's value in the document contains the needle (recursively).
pub(crate) fn doc_matches(doc: &bson::Document, needle: &str) -> bool {
    doc.iter().any(|(_, value)| value_matches(value, needle))
}

/// Search every collection in a database for documents containing `term` anywhere
/// in their values (case-insensitive substring, recursing into nested documents
/// and arrays), the way Studio-3T's "Search in…" does. Each collection is scanned
/// up to `scan_limit` documents and reports up to `max_hits` matches. Returns only
/// collections with at least one match.
#[tauri::command]
pub async fn search_collections(
    ctx: State<'_, AppContext>,
    id: String,
    database: String,
    term: String,
    scan_limit: Option<i64>,
    max_hits: Option<i64>,
) -> Result<Vec<CollectionHits>, AppError> {
    let needle = term.trim().to_lowercase();
    if needle.is_empty() {
        return Ok(Vec::new());
    }
    let client = match ctx.client(&id).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let db = client.database(&database);

    let scan = match scan_limit {
        Some(val) if val > 0 => val.min(MAX_SCAN),
        _ => DEFAULT_SCAN,
    };
    let hit_cap = match max_hits {
        Some(val) if val > 0 => val as usize,
        _ => DEFAULT_MAX_HITS as usize,
    };

    let names = match db.list_collection_names().await {
        Ok(val) => val,
        Err(e) => return Err(AppError::Mongo(e)),
    };

    let mut results: Vec<CollectionHits> = Vec::new();
    for name in names {
        let col = db.collection::<bson::Document>(&name);
        let mut cursor = match col.find(bson::doc! {}).limit(scan).max_time(MAX_QUERY_TIME).await {
            Ok(val) => val,
            Err(e) => return Err(AppError::Mongo(e)),
        };
        let mut scanned: u64 = 0;
        let mut matched: u64 = 0;
        let mut hits: Vec<serde_json::Value> = Vec::new();
        loop {
            let doc: bson::Document = match next_document(&mut cursor).await {
                Ok(Some(value)) => value,
                Ok(None) => break,
                Err(e) => return Err(e),
            };
            scanned += 1;
            if doc_matches(&doc, &needle) {
                matched += 1;
                if hits.len() < hit_cap {
                    hits.push(serde_json::Value::from(bson::Bson::Document(doc)));
                }
            }
        }
        if matched > 0 {
            results.push(CollectionHits {
                collection: name,
                scanned: scanned,
                matched: matched,
                hits: hits,
            });
        }
    }
    Ok(results)
}

#[cfg(test)]
mod tests;
