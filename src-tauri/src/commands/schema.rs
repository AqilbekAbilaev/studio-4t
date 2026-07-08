use crate::error::AppError;
use mongodb::bson;
use serde::Serialize;
use std::collections::BTreeMap;
use tauri::State;

use super::{collect_documents, MAX_QUERY_TIME, AppContext};

// The number of documents sampled by default when the frontend does not supply
// an explicit size. Clamped hard limits keep a huge collection from stalling the
// UI while still giving a representative picture.
const DEFAULT_SAMPLE_SIZE: i64 = 1000;
const MIN_SAMPLE_SIZE: i64 = 1;
const MAX_SAMPLE_SIZE: i64 = 10_000;

// How many distinct type/coverage counts to accumulate. A field with the same
// path may hold different BSON types across documents (a classic schema smell),
// so each field carries a per-type breakdown.
#[derive(Serialize)]
pub struct TypeCount {
    pub bson_type: String,
    pub count: u64,
}

#[derive(Serialize)]
pub struct FieldSchema {
    // Dotted path, e.g. "address.city". Array-of-subdocument fields contribute
    // both the array field itself and its element sub-fields under the same path.
    pub path: String,
    // How many sampled documents contain this path at least once.
    pub present: u64,
    // Per-type occurrence counts (counted once per document), sorted descending.
    pub types: Vec<TypeCount>,
}

#[derive(Serialize)]
pub struct SchemaReport {
    pub sampled: u64,
    pub fields: Vec<FieldSchema>,
}

// Human-readable BSON type name, matching the vocabulary the mongo shell uses so
// the UI reads familiarly to anyone who knows MongoDB.
fn bson_type_name(value: &bson::Bson) -> &'static str {
    match value {
        bson::Bson::Double(_) => "double",
        bson::Bson::String(_) => "string",
        bson::Bson::Document(_) => "object",
        bson::Bson::Array(_) => "array",
        bson::Bson::Binary(_) => "binData",
        bson::Bson::ObjectId(_) => "objectId",
        bson::Bson::Boolean(_) => "bool",
        bson::Bson::DateTime(_) => "date",
        bson::Bson::Null => "null",
        bson::Bson::RegularExpression(_) => "regex",
        bson::Bson::JavaScriptCode(_) => "javascript",
        bson::Bson::JavaScriptCodeWithScope(_) => "javascriptWithScope",
        bson::Bson::Int32(_) => "int",
        bson::Bson::Int64(_) => "long",
        bson::Bson::Timestamp(_) => "timestamp",
        bson::Bson::Decimal128(_) => "decimal",
        bson::Bson::MinKey => "minKey",
        bson::Bson::MaxKey => "maxKey",
        bson::Bson::Undefined => "undefined",
        bson::Bson::DbPointer(_) => "dbPointer",
        bson::Bson::Symbol(_) => "symbol",
    }
}

// Record every path found in one document into `seen`, mapping a dotted path to
// the set of BSON type names it took *within this document*. Counting per
// document (rather than per occurrence) means an array of 50 sub-documents does
// not inflate coverage — the field is simply "present" in this one document.
fn collect_paths(doc: &bson::Document, prefix: &str, seen: &mut BTreeMap<String, Vec<String>>) {
    for (key, value) in doc {
        let path = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        record_type(&path, value, seen);
        match value {
            bson::Bson::Document(sub) => {
                collect_paths(sub, &path, seen);
            }
            bson::Bson::Array(items) => {
                // Recurse into array elements that are sub-documents so an array of
                // objects surfaces its element shape as `path.subfield`, matching how
                // Studio-3T flattens embedded arrays in its schema explorer.
                for item in items {
                    if let bson::Bson::Document(sub) = item {
                        collect_paths(sub, &path, seen);
                    }
                }
            }
            _ => {}
        }
    }
}

// Add a type name to a path's per-document set, de-duplicating so a path seen
// several times in one document (e.g. across array elements) counts each type
// only once for this document.
fn record_type(path: &str, value: &bson::Bson, seen: &mut BTreeMap<String, Vec<String>>) {
    let type_name = bson_type_name(value).to_string();
    let entry = seen.entry(path.to_string()).or_insert_with(Vec::new);
    if !entry.iter().any(|existing| existing == &type_name) {
        entry.push(type_name);
    }
}

// Pure inference over a sample of documents. Kept free of any I/O so it can be
// unit-tested directly. Returns fields sorted by path, each with its coverage
// and per-type breakdown (types sorted most-common first).
pub(crate) fn infer_schema(docs: &[bson::Document]) -> SchemaReport {
    // path -> present-count, and path -> (type -> count).
    let mut present: BTreeMap<String, u64> = BTreeMap::new();
    let mut type_counts: BTreeMap<String, BTreeMap<String, u64>> = BTreeMap::new();

    for doc in docs {
        let mut seen: BTreeMap<String, Vec<String>> = BTreeMap::new();
        collect_paths(doc, "", &mut seen);
        for (path, types) in seen {
            let present_entry = present.entry(path.clone()).or_insert(0);
            *present_entry += 1;
            let counts = type_counts.entry(path).or_insert_with(BTreeMap::new);
            for type_name in types {
                let count_entry = counts.entry(type_name).or_insert(0);
                *count_entry += 1;
            }
        }
    }

    let mut fields: Vec<FieldSchema> = Vec::new();
    for (path, present_count) in present {
        let counts = match type_counts.get(&path) {
            Some(val) => val,
            None => continue,
        };
        let mut types: Vec<TypeCount> = counts
            .iter()
            .map(|(name, count)| TypeCount {
                bson_type: name.clone(),
                count: *count,
            })
            .collect();
        // Most common type first; ties fall back to type name for a stable order.
        types.sort_by(|a, b| {
            b.count
                .cmp(&a.count)
                .then_with(|| a.bson_type.cmp(&b.bson_type))
        });
        fields.push(FieldSchema {
            path: path,
            present: present_count,
            types: types,
        });
    }

    SchemaReport {
        sampled: docs.len() as u64,
        fields: fields,
    }
}

/// Sample documents from a collection and infer its schema: for every field
/// (nested and array-embedded paths flattened with dot notation) report how many
/// sampled documents contain it and the distribution of BSON types it holds.
/// Uses `$sample` for a representative random draw and the shared query-time cap
/// so a huge collection can't hang the UI.
#[tauri::command]
pub async fn analyze_schema(
    ctx: State<'_, AppContext>,
    id: String,
    database: String,
    collection: String,
    sample_size: Option<i64>,
) -> Result<SchemaReport, AppError> {
    let requested = match sample_size {
        Some(val) => val,
        None => DEFAULT_SAMPLE_SIZE,
    };
    let size = requested.clamp(MIN_SAMPLE_SIZE, MAX_SAMPLE_SIZE);

    let col = match ctx.collection(&id, &database, &collection).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };

    let pipeline = vec![bson::doc! { "$sample": { "size": size } }];
    let mut cursor = match col.aggregate(pipeline).max_time(MAX_QUERY_TIME).await {
        Ok(val) => val,
        Err(e) => return Err(AppError::Mongo(e)),
    };

    let docs = match collect_documents(&mut cursor).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };

    Ok(infer_schema(&docs))
}

#[cfg(test)]
mod tests;
