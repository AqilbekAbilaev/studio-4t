use crate::error::AppError;
use crate::pool::ConnectionPool;
use crate::storage::Storage;
use mongodb::bson;
use mongodb::Client;
use mongodb::Collection;
use serde::Serialize;
use std::io::Write;
use std::time::Duration;

pub mod connection;
pub mod query;
pub mod admin;
pub mod persistence;
pub mod shell;
pub mod schema;
pub mod sql;
pub mod masking;
pub mod stats;
pub mod duplicate;
pub mod serverinfo;
pub mod migration;
pub mod search;
pub mod gridfs;
pub mod users;
pub mod functions;
pub mod mapreduce;
pub mod copyops;
pub mod compare;
pub mod folders;

pub use connection::*;
pub use query::*;
pub use admin::*;
pub use persistence::*;
pub use shell::*;
pub use schema::*;
pub use sql::*;
pub use masking::*;
pub use stats::*;
pub use duplicate::*;
pub use serverinfo::*;
pub use migration::*;
pub use search::*;
pub use gridfs::*;
pub use users::*;
pub use functions::*;
pub use mapreduce::*;
pub use copyops::*;
pub use compare::*;
pub use folders::*;

// Server-side time cap on user queries so a runaway find/aggregate aborts on the
// server instead of hanging the UI (Tauri commands can't be cancelled in-flight).
pub(crate) const MAX_QUERY_TIME: Duration = Duration::from_secs(60);

/// Resolve the live MongoDB client for a saved connection: look up its config and
/// hand off to the pool, which caches the client and reads credentials from the
/// keychain only when it actually opens a new connection. Every command that
/// operates on a connection goes through here, so the config-lookup + connect
/// dance lives in exactly one place (and the keychain read stays off the hot path).
pub(crate) async fn client_for(
    pool: &ConnectionPool,
    storage: &Storage,
    id: &str,
) -> Result<Client, AppError> {
    let config = match storage.find(id) {
        Some(val) => val,
        None => return Err(AppError::UnknownConnection(id.to_string())),
    };
    pool.connect(&config).await
}

#[derive(Serialize)]
pub struct DatabaseInfo {
    pub name: String,
    pub collections: Vec<String>,
    pub accessible: bool,
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
pub(crate) fn parse_ejson_document(ejson: &str) -> Result<bson::Document, AppError> {
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
pub(crate) fn parse_pipeline(pipeline: &str) -> Result<Vec<bson::Document>, AppError> {
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
pub(crate) fn parse_json_documents(text: &str) -> Result<Vec<bson::Document>, AppError> {
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

// Adds any of `doc`'s keys not already present to `headers`, in first-seen order.
// Called once per document while building the CSV header union.
fn csv_collect_headers(headers: &mut Vec<String>, doc: &bson::Document) {
    for (key, _) in doc {
        if !headers.iter().any(|existing| existing == key) {
            headers.push(key.clone());
        }
    }
}

// One CSV row (in `headers` column order) for a document; a key the document
// lacks becomes an empty cell.
fn csv_format_row(headers: &[String], doc: &bson::Document) -> String {
    let row: Vec<String> = headers
        .iter()
        .map(|header| match doc.get(header) {
            Some(value) => csv_escape(&bson_to_csv_cell(value)),
            None => String::new(),
        })
        .collect();
    row.join(",")
}

// Pretty-prints one document as an element of a JSON array, prefixed with the
// separator for its position (the first element has none). Shared by the
// streaming exporter and the test-only `docs_to_json_array`, so the streamed and
// the tested output are byte-identical.
fn json_array_element(doc: &bson::Document, first: bool) -> Result<String, AppError> {
    let value = serde_json::Value::from(bson::Bson::Document(doc.clone()));
    let pretty = match serde_json::to_string_pretty(&value) {
        Ok(val) => val,
        Err(e) => return Err(AppError::Serde(e)),
    };
    let prefix = if first { "\n" } else { ",\n" };
    Ok(format!("{}{}", prefix, pretty))
}

// Buffered whole-slice assemblers built from the same primitives the streaming
// exporter uses. Compiled in test builds only — the app streams via
// `stream_export`; these exist so the CSV/JSON formatting can be unit-tested
// without a live MongoDB cursor.
#[cfg(test)]
pub(crate) fn docs_to_csv(docs: &[bson::Document]) -> String {
    let mut headers: Vec<String> = Vec::new();
    for doc in docs {
        csv_collect_headers(&mut headers, doc);
    }
    let mut out = String::new();
    let header_line: Vec<String> = headers.iter().map(|h| csv_escape(h)).collect();
    out.push_str(&header_line.join(","));
    out.push('\n');
    for doc in docs {
        out.push_str(&csv_format_row(&headers, doc));
        out.push('\n');
    }
    out
}

#[cfg(test)]
pub(crate) fn docs_to_json_array(docs: &[bson::Document]) -> Result<String, AppError> {
    let mut out = String::from("[");
    for (index, doc) in docs.iter().enumerate() {
        let element = match json_array_element(doc, index == 0) {
            Ok(val) => val,
            Err(e) => return Err(e),
        };
        out.push_str(&element);
    }
    if !docs.is_empty() {
        out.push('\n');
    }
    out.push(']');
    Ok(out)
}

// Writes `bytes` to `writer`, mapping any I/O error to `AppError`. Keeps the
// streaming exporter free of repeated match blocks.
fn write_bytes<W: Write>(writer: &mut W, bytes: &[u8]) -> Result<(), AppError> {
    match writer.write_all(bytes) {
        Ok(_) => Ok(()),
        Err(e) => Err(AppError::Io(e)),
    }
}

// Opens a fresh cursor for one export pass, applying the optional server-side
// time cap and row limit. A separate function so the CSV two-pass path can
// re-open an identical cursor for its second scan.
async fn export_cursor(
    col: &Collection<bson::Document>,
    filter: bson::Document,
    limit: Option<i64>,
    max_time: Option<Duration>,
) -> Result<mongodb::Cursor<bson::Document>, AppError> {
    let mut query = col.find(filter);
    if let Some(duration) = max_time {
        query = query.max_time(duration);
    }
    if let Some(value) = limit {
        if value > 0 {
            query = query.limit(value);
        }
    }
    match query.await {
        Ok(val) => Ok(val),
        Err(e) => Err(AppError::Mongo(e)),
    }
}

/// Streams a collection to `path` as JSON or CSV without ever holding the whole
/// result set in memory: documents are read from the cursor one at a time,
/// transformed, and written straight to a buffered file. `transform` lets the
/// masked export apply its rules; plain export passes a no-op. Returns the number
/// of documents written.
///
/// JSON is a single streaming pass. CSV needs the full header union up front, so
/// it makes two passes over the collection (pass 1 collects headers, pass 2 writes
/// rows) — this assumes the collection isn't mutated between the passes.
/// `transform` runs in both CSV passes because a rule can drop a key, which must
/// be reflected in the header union.
pub(crate) async fn stream_export<F>(
    col: &Collection<bson::Document>,
    filter: bson::Document,
    limit: Option<i64>,
    max_time: Option<Duration>,
    path: &str,
    format: &str,
    mut transform: F,
) -> Result<usize, AppError>
where
    F: FnMut(&mut bson::Document) -> Result<(), AppError>,
{
    if format == "csv" {
        return stream_export_csv(col, filter, limit, max_time, path, &mut transform).await;
    }
    let file = match std::fs::File::create(path) {
        Ok(val) => val,
        Err(e) => return Err(AppError::Io(e)),
    };
    let mut writer = std::io::BufWriter::new(file);
    match write_bytes(&mut writer, b"[") {
        Ok(_) => {}
        Err(e) => return Err(e),
    }
    let mut cursor = match export_cursor(col, filter, limit, max_time).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let mut count: usize = 0;
    loop {
        let has_next = match cursor.advance().await {
            Ok(val) => val,
            Err(e) => return Err(AppError::Mongo(e)),
        };
        if !has_next {
            break;
        }
        let mut doc: bson::Document = match cursor.deserialize_current() {
            Ok(val) => val,
            Err(e) => return Err(AppError::Mongo(e)),
        };
        match transform(&mut doc) {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        let element = match json_array_element(&doc, count == 0) {
            Ok(val) => val,
            Err(e) => return Err(e),
        };
        match write_bytes(&mut writer, element.as_bytes()) {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        count += 1;
    }
    if count > 0 {
        match write_bytes(&mut writer, b"\n") {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
    }
    match write_bytes(&mut writer, b"]") {
        Ok(_) => {}
        Err(e) => return Err(e),
    }
    match writer.flush() {
        Ok(_) => Ok(count),
        Err(e) => Err(AppError::Io(e)),
    }
}

// CSV branch of `stream_export`: two passes (headers, then rows). Split out to
// keep `stream_export` readable.
async fn stream_export_csv<F>(
    col: &Collection<bson::Document>,
    filter: bson::Document,
    limit: Option<i64>,
    max_time: Option<Duration>,
    path: &str,
    transform: &mut F,
) -> Result<usize, AppError>
where
    F: FnMut(&mut bson::Document) -> Result<(), AppError>,
{
    // Pass 1: header union (transform applied, since a rule can drop keys).
    let mut headers: Vec<String> = Vec::new();
    let mut cursor = match export_cursor(col, filter.clone(), limit, max_time).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    loop {
        let has_next = match cursor.advance().await {
            Ok(val) => val,
            Err(e) => return Err(AppError::Mongo(e)),
        };
        if !has_next {
            break;
        }
        let mut doc: bson::Document = match cursor.deserialize_current() {
            Ok(val) => val,
            Err(e) => return Err(AppError::Mongo(e)),
        };
        match transform(&mut doc) {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        csv_collect_headers(&mut headers, &doc);
    }
    // Pass 2: header line, then one row per document.
    let file = match std::fs::File::create(path) {
        Ok(val) => val,
        Err(e) => return Err(AppError::Io(e)),
    };
    let mut writer = std::io::BufWriter::new(file);
    let header_line: Vec<String> = headers.iter().map(|h| csv_escape(h)).collect();
    let mut header_out = header_line.join(",");
    header_out.push('\n');
    match write_bytes(&mut writer, header_out.as_bytes()) {
        Ok(_) => {}
        Err(e) => return Err(e),
    }
    let mut count: usize = 0;
    let mut cursor = match export_cursor(col, filter, limit, max_time).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    loop {
        let has_next = match cursor.advance().await {
            Ok(val) => val,
            Err(e) => return Err(AppError::Mongo(e)),
        };
        if !has_next {
            break;
        }
        let mut doc: bson::Document = match cursor.deserialize_current() {
            Ok(val) => val,
            Err(e) => return Err(AppError::Mongo(e)),
        };
        match transform(&mut doc) {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        let mut row = csv_format_row(&headers, &doc);
        row.push('\n');
        match write_bytes(&mut writer, row.as_bytes()) {
            Ok(_) => {}
            Err(e) => return Err(e),
        }
        count += 1;
    }
    match writer.flush() {
        Ok(_) => Ok(count),
        Err(e) => Err(AppError::Io(e)),
    }
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

pub(crate) fn csv_to_docs(text: &str) -> Vec<bson::Document> {
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

#[cfg(test)]
mod tests;
