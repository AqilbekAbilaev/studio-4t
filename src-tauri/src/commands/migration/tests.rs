use super::{generate_sql, infer_columns, sql_value};
use mongodb::bson::{doc, Bson};

fn col_type(cols: &[super::Column], name: &str) -> String {
    cols.iter().find(|c| c.name == name).expect("column present").sql_type.clone()
}

#[test]
fn infers_basic_types_in_first_seen_order() {
    let docs = vec![doc! { "name": "a", "age": 30_i32, "active": true }];
    let cols = infer_columns(&docs);
    let names: Vec<&str> = cols.iter().map(|c| c.name.as_str()).collect();
    assert_eq!(names, vec!["name", "age", "active"]);
    assert_eq!(col_type(&cols, "name"), "TEXT");
    assert_eq!(col_type(&cols, "age"), "INTEGER");
    assert_eq!(col_type(&cols, "active"), "BOOLEAN");
}

#[test]
fn widens_mixed_numeric_columns() {
    let docs = vec![
        doc! { "n": 1_i32 },
        doc! { "n": 2_i64 },
        doc! { "n": 3.5_f64 },
    ];
    let cols = infer_columns(&docs);
    assert_eq!(col_type(&cols, "n"), "DOUBLE PRECISION");
}

#[test]
fn incompatible_types_fall_back_to_text() {
    let docs = vec![doc! { "x": 1_i32 }, doc! { "x": "str" }];
    let cols = infer_columns(&docs);
    assert_eq!(col_type(&cols, "x"), "TEXT");
}

#[test]
fn null_only_column_is_text() {
    let docs = vec![doc! { "maybe": Bson::Null }, doc! { "maybe": Bson::Null }];
    let cols = infer_columns(&docs);
    assert_eq!(col_type(&cols, "maybe"), "TEXT");
}

#[test]
fn null_does_not_override_a_real_type() {
    let docs = vec![doc! { "v": 5_i32 }, doc! { "v": Bson::Null }];
    let cols = infer_columns(&docs);
    assert_eq!(col_type(&cols, "v"), "INTEGER");
}

#[test]
fn objectid_and_date_types() {
    let oid = mongodb::bson::oid::ObjectId::new();
    let docs = vec![doc! { "_id": oid, "when": mongodb::bson::DateTime::now() }];
    let cols = infer_columns(&docs);
    assert_eq!(col_type(&cols, "_id"), "VARCHAR(24)");
    assert_eq!(col_type(&cols, "when"), "TIMESTAMP");
}

#[test]
fn sql_value_escapes_strings() {
    assert_eq!(sql_value(&Bson::String("O'Brien".to_string())), "'O''Brien'");
}

#[test]
fn sql_value_scalars() {
    assert_eq!(sql_value(&Bson::Int32(7)), "7");
    assert_eq!(sql_value(&Bson::Boolean(true)), "TRUE");
    assert_eq!(sql_value(&Bson::Boolean(false)), "FALSE");
    assert_eq!(sql_value(&Bson::Null), "NULL");
}

#[test]
fn sql_value_nested_becomes_json_string() {
    let v = Bson::Document(doc! { "a": 1_i32 });
    let out = sql_value(&v);
    assert!(out.starts_with('\'') && out.ends_with('\''));
    assert!(out.contains("\"a\""));
}

#[test]
fn generate_sql_has_create_and_inserts() {
    let docs = vec![
        doc! { "name": "Alice", "age": 30_i32 },
        doc! { "name": "Bob", "age": 25_i32 },
    ];
    let cols = infer_columns(&docs);
    let sql = generate_sql("users", &cols, &docs);
    assert!(sql.contains("CREATE TABLE \"users\" ("));
    assert!(sql.contains("\"name\" TEXT"));
    assert!(sql.contains("\"age\" INTEGER"));
    assert!(sql.contains("INSERT INTO \"users\" (\"name\", \"age\") VALUES ('Alice', 30);"));
    assert!(sql.contains("VALUES ('Bob', 25);"));
}

#[test]
fn missing_field_inserts_null() {
    let docs = vec![doc! { "a": 1_i32, "b": 2_i32 }, doc! { "a": 3_i32 }];
    let cols = infer_columns(&docs);
    let sql = generate_sql("t", &cols, &docs);
    assert!(sql.contains("VALUES (3, NULL);"));
}

#[test]
fn identifiers_are_quoted_and_escaped() {
    let docs = vec![doc! { "we\"ird": 1_i32 }];
    let cols = infer_columns(&docs);
    let sql = generate_sql("tab", &cols, &docs);
    assert!(sql.contains("\"we\"\"ird\""));
}
