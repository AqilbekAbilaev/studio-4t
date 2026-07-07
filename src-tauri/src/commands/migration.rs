use crate::error::AppError;
use crate::pool::ConnectionPool;
use crate::storage::Storage;
use mongodb::bson;
use tauri::State;

use super::{collect_documents, MAX_QUERY_TIME};

const DEFAULT_SAMPLE: i64 = 1000;
const MAX_SAMPLE: i64 = 10_000;

// One inferred relational column.
pub(crate) struct Column {
    pub name: String,
    pub sql_type: String,
}

// The SQL type for a single BSON value. Nested documents/arrays are stored as
// JSON text, which keeps the migration lossless without exploding into extra
// tables (a deliberate v1 scope choice).
fn sql_type_for(value: &bson::Bson) -> &'static str {
    match value {
        bson::Bson::String(_) => "TEXT",
        bson::Bson::Int32(_) => "INTEGER",
        bson::Bson::Int64(_) => "BIGINT",
        bson::Bson::Double(_) => "DOUBLE PRECISION",
        bson::Bson::Boolean(_) => "BOOLEAN",
        bson::Bson::DateTime(_) => "TIMESTAMP",
        bson::Bson::ObjectId(_) => "VARCHAR(24)",
        bson::Bson::Decimal128(_) => "DECIMAL",
        bson::Bson::Document(_) | bson::Bson::Array(_) => "TEXT",
        _ => "TEXT",
    }
}

// Numeric widening rank so a column that mixes INTEGER and DOUBLE across documents
// resolves to the widest numeric type instead of collapsing to TEXT. Non-numeric
// types are rank 0 (not widenable).
fn numeric_rank(sql_type: &str) -> u8 {
    match sql_type {
        "INTEGER" => 1,
        "BIGINT" => 2,
        "DOUBLE PRECISION" => 3,
        "DECIMAL" => 4,
        _ => 0,
    }
}

// Combine the type seen so far with a new observation. Same type stays; two
// numerics widen to the larger; any other disagreement falls back to TEXT.
fn unify(existing: &str, new: &str) -> String {
    if existing == new {
        return existing.to_string();
    }
    let (a, b) = (numeric_rank(existing), numeric_rank(new));
    if a > 0 && b > 0 {
        return if a >= b { existing.to_string() } else { new.to_string() };
    }
    "TEXT".to_string()
}

// Infer columns in first-seen order across the sampled documents. NULLs and
// absent fields don't drive the type; a field seen only as null becomes TEXT.
pub(crate) fn infer_columns(docs: &[bson::Document]) -> Vec<Column> {
    let mut order: Vec<String> = Vec::new();
    let mut types: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    for doc in docs {
        for (key, value) in doc {
            if !order.iter().any(|existing| existing == key) {
                order.push(key.clone());
            }
            if matches!(value, bson::Bson::Null) {
                continue;
            }
            let observed = sql_type_for(value).to_string();
            let resolved = match types.get(key) {
                Some(prev) => unify(prev, &observed),
                None => observed,
            };
            types.insert(key.clone(), resolved);
        }
    }
    order
        .into_iter()
        .map(|name| {
            let sql_type = match types.get(&name) {
                Some(val) => val.clone(),
                None => "TEXT".to_string(),
            };
            Column { name: name, sql_type: sql_type }
        })
        .collect()
}

// Double-quote a SQL identifier, escaping embedded quotes.
fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

// Single-quote a SQL string literal, doubling embedded quotes (standard SQL).
fn quote_str(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

// Render one BSON value as a SQL literal for an INSERT.
pub(crate) fn sql_value(value: &bson::Bson) -> String {
    match value {
        bson::Bson::Null => "NULL".to_string(),
        bson::Bson::String(val) => quote_str(val),
        bson::Bson::Int32(val) => val.to_string(),
        bson::Bson::Int64(val) => val.to_string(),
        bson::Bson::Double(val) => {
            if val.is_finite() {
                val.to_string()
            } else {
                "NULL".to_string()
            }
        }
        bson::Bson::Boolean(val) => if *val { "TRUE".to_string() } else { "FALSE".to_string() },
        bson::Bson::ObjectId(val) => quote_str(&val.to_hex()),
        bson::Bson::Decimal128(val) => val.to_string(),
        bson::Bson::DateTime(val) => match val.try_to_rfc3339_string() {
            Ok(text) => quote_str(&text),
            Err(_) => "NULL".to_string(),
        },
        other => {
            // Documents, arrays, and anything exotic become JSON text.
            let json = serde_json::Value::from(other.clone());
            quote_str(&json.to_string())
        }
    }
}

// Build the full CREATE TABLE + INSERT script for the given table name, inferred
// columns, and documents. A field missing from a given document inserts NULL.
pub(crate) fn generate_sql(table: &str, columns: &[Column], docs: &[bson::Document]) -> String {
    let mut out = String::new();
    out.push_str(&format!("CREATE TABLE {} (\n", quote_ident(table)));
    let col_defs: Vec<String> = columns
        .iter()
        .map(|col| format!("  {} {}", quote_ident(&col.name), col.sql_type))
        .collect();
    out.push_str(&col_defs.join(",\n"));
    out.push_str("\n);\n");

    if columns.is_empty() {
        return out;
    }

    let col_list: Vec<String> = columns.iter().map(|col| quote_ident(&col.name)).collect();
    let col_clause = col_list.join(", ");
    for doc in docs {
        let values: Vec<String> = columns
            .iter()
            .map(|col| match doc.get(&col.name) {
                Some(value) => sql_value(value),
                None => "NULL".to_string(),
            })
            .collect();
        out.push_str(&format!(
            "INSERT INTO {} ({}) VALUES ({});\n",
            quote_ident(table),
            col_clause,
            values.join(", ")
        ));
    }
    out
}

/// Generate a SQL migration script (CREATE TABLE + INSERT statements) from a
/// collection, the way Studio-3T's SQL Migration does. Samples up to `limit`
/// documents, infers a relational schema (nested values stored as JSON text), and
/// returns the script. `table_name` defaults to the collection name.
#[tauri::command]
pub async fn generate_sql_migration(
    pool: State<'_, ConnectionPool>,
    storage: State<'_, Storage>,
    id: String,
    database: String,
    collection: String,
    table_name: Option<String>,
    limit: Option<i64>,
) -> Result<String, AppError> {
    let client = match super::client_for(&pool, &storage, &id).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };
    let col = client
        .database(&database)
        .collection::<bson::Document>(&collection);

    let requested = match limit {
        Some(val) if val > 0 => val,
        _ => DEFAULT_SAMPLE,
    };
    let capped = requested.min(MAX_SAMPLE);

    let mut cursor = match col.find(bson::doc! {}).limit(capped).max_time(MAX_QUERY_TIME).await {
        Ok(val) => val,
        Err(e) => return Err(AppError::Mongo(e)),
    };
    let docs = match collect_documents(&mut cursor).await {
        Ok(val) => val,
        Err(e) => return Err(e),
    };

    let table = match table_name {
        Some(val) if !val.trim().is_empty() => val.trim().to_string(),
        _ => collection.clone(),
    };
    let columns = infer_columns(&docs);
    Ok(generate_sql(&table, &columns, &docs))
}

#[cfg(test)]
mod tests;
