use super::*;

// The frontend's query parser emits canonical Extended JSON; these confirm the Rust
// decode (the same `serde_json::from_str::<bson::Bson>` the commands use) yields the
// right BSON types for the tricky cases.

#[test]
fn ejson_objectid_decodes_to_object_id() {
    let doc = parse_ejson_document(r#"{"_id":{"$oid":"507f1f77bcf86cd799439011"}}"#).unwrap();
    assert!(matches!(doc.get("_id"), Some(bson::Bson::ObjectId(_))));
}

#[test]
fn ejson_date_decodes_to_datetime() {
    let doc =
        parse_ejson_document(r#"{"created":{"$date":{"$numberLong":"1704067200000"}}}"#).unwrap();
    assert!(matches!(doc.get("created"), Some(bson::Bson::DateTime(_))));
}

#[test]
fn ejson_regex_decodes_to_regular_expression() {
    let doc =
        parse_ejson_document(r#"{"name":{"$regularExpression":{"pattern":"^jo","options":"i"}}}"#)
            .unwrap();
    assert!(matches!(doc.get("name"), Some(bson::Bson::RegularExpression(_))));
}

#[test]
fn ejson_nested_operator_number_decodes_to_int32() {
    let doc = parse_ejson_document(r#"{"age":{"$gt":{"$numberInt":"18"}}}"#).unwrap();
    let inner = doc.get_document("age").unwrap();
    assert!(matches!(inner.get("$gt"), Some(bson::Bson::Int32(18))));
}

#[test]
fn ejson_string_with_punctuation_is_preserved() {
    let doc = parse_ejson_document(r#"{"note":"hello, world: x"}"#).unwrap();
    match doc.get("note") {
        Some(bson::Bson::String(value)) => assert_eq!(value, "hello, world: x"),
        other => panic!("expected a string, got {:?}", other),
    }
}

#[test]
fn empty_and_braces_decode_to_empty_document() {
    assert!(parse_ejson_document("").unwrap().is_empty());
    assert!(parse_ejson_document("{}").unwrap().is_empty());
}

#[test]
fn non_object_ejson_is_an_error() {
    assert!(parse_ejson_document("[1, 2, 3]").is_err());
}

#[test]
fn smart_quotes_are_normalized_as_a_backstop() {
    let doc = parse_ejson_document("{\u{201C}name\u{201D}:\u{201C}John\u{201D}}").unwrap();
    match doc.get("name") {
        Some(bson::Bson::String(value)) => assert_eq!(value, "John"),
        other => panic!("expected a string, got {:?}", other),
    }
}

#[test]
fn pipeline_canonical_ejson_decodes_to_stage_documents() {
    let stages = parse_pipeline(
        r#"[{"$match":{"x":{"$numberInt":"1"}}},{"$group":{"_id":"$y","n":{"$sum":{"$numberInt":"1"}}}}]"#,
    )
    .unwrap();
    assert_eq!(stages.len(), 2);
    assert!(stages[0].contains_key("$match"));
}

#[test]
fn empty_pipeline_decodes_to_no_stages() {
    assert!(parse_pipeline("[]").unwrap().is_empty());
    assert!(parse_pipeline("").unwrap().is_empty());
}

#[test]
fn only_the_id_index_is_protected() {
    // The `_id_` index can never be dropped or hidden; every other name is fair game,
    // including near-misses that merely contain `_id`.
    assert!(is_protected_index("_id_"));
    assert!(!is_protected_index("email_1"));
    assert!(!is_protected_index("_id_2"));
    assert!(!is_protected_index("user_id_1"));
    assert!(!is_protected_index(""));
}

#[test]
fn sort_document_preserves_key_order() {
    // The link the plan was least sure of: JS EJSON.stringify keeps key order, and
    // bson::Document must keep it through serde_json -> BSON so sort fields apply in
    // the order the user wrote them.
    let doc = parse_ejson_document(r#"{"a":{"$numberInt":"1"},"b":{"$numberInt":"-1"}}"#).unwrap();
    let keys: Vec<&str> = doc.keys().map(|k| k.as_str()).collect();
    assert_eq!(keys, vec!["a", "b"]);

    let reversed = parse_ejson_document(r#"{"b":{"$numberInt":"1"},"a":{"$numberInt":"1"}}"#).unwrap();
    let reversed_keys: Vec<&str> = reversed.keys().map(|k| k.as_str()).collect();
    assert_eq!(reversed_keys, vec!["b", "a"]);
}
