use super::doc_matches;
use mongodb::bson::doc;

// The needle is always pre-lowercased by the caller; these pass lowercase.

#[test]
fn matches_string_value_case_insensitively() {
    let d = doc! { "name": "Alice Smith" };
    assert!(doc_matches(&d, "alice"));
    assert!(doc_matches(&d, "smith"));
    assert!(!doc_matches(&d, "bob"));
}

#[test]
fn matches_substring() {
    let d = doc! { "email": "person@example.com" };
    assert!(doc_matches(&d, "example"));
    assert!(doc_matches(&d, "@example.com"));
}

#[test]
fn matches_numeric_text() {
    let d = doc! { "age": 42_i32, "score": 3.14_f64 };
    assert!(doc_matches(&d, "42"));
    assert!(doc_matches(&d, "3.14"));
    assert!(!doc_matches(&d, "99"));
}

#[test]
fn matches_boolean_text() {
    let d = doc! { "active": true };
    assert!(doc_matches(&d, "true"));
    assert!(!doc_matches(&d, "false"));
}

#[test]
fn recurses_into_nested_documents() {
    let d = doc! { "address": { "city": "Portland" } };
    assert!(doc_matches(&d, "portland"));
}

#[test]
fn recurses_into_arrays() {
    let d = doc! { "tags": ["red", "green", "blue"] };
    assert!(doc_matches(&d, "green"));
    assert!(!doc_matches(&d, "purple"));
}

#[test]
fn recurses_into_array_of_documents() {
    let d = doc! { "items": [ { "sku": "ABC" }, { "sku": "XYZ" } ] };
    assert!(doc_matches(&d, "xyz"));
}

#[test]
fn no_match_returns_false() {
    let d = doc! { "a": 1_i32, "b": "hello" };
    assert!(!doc_matches(&d, "zzz"));
}

#[test]
fn matches_objectid_hex() {
    let oid = mongodb::bson::oid::ObjectId::new();
    let hex = oid.to_hex();
    let d = doc! { "_id": oid };
    assert!(doc_matches(&d, &hex[..8]));
}
