use super::*;

#[test]
fn to_document_parses_plain_object() {
    let value = serde_json::json!({ "a": 1, "b": "x" });
    let doc = to_document(&value).unwrap();
    assert!(doc.contains_key("a"));
    assert_eq!(doc.get_str("b").unwrap(), "x");
}

#[test]
fn to_document_decodes_objectid_ejson() {
    // The shell's ObjectId("…") constructor produces { $oid: "…" }; it must
    // round-trip to a real BSON ObjectId, like the find/aggregate commands.
    let value = serde_json::json!({ "_id": { "$oid": "507f1f77bcf86cd799439011" } });
    let doc = to_document(&value).unwrap();
    match doc.get("_id") {
        Some(bson::Bson::ObjectId(oid)) => {
            assert_eq!(oid.to_hex(), "507f1f77bcf86cd799439011")
        }
        other => panic!("expected ObjectId, got {:?}", other),
    }
}

#[test]
fn to_document_treats_null_as_empty() {
    let doc = to_document(&serde_json::Value::Null).unwrap();
    assert!(doc.is_empty());
}

#[test]
fn to_document_rejects_non_objects() {
    assert!(to_document(&serde_json::json!([1, 2, 3])).is_err());
    assert!(to_document(&serde_json::json!(5)).is_err());
    assert!(to_document(&serde_json::json!("hello")).is_err());
}

#[test]
fn arg_doc_defaults_to_empty_when_missing() {
    let args: Vec<serde_json::Value> = Vec::new();
    let doc = arg_doc(&args, 0).unwrap();
    assert!(doc.is_empty());
}

#[test]
fn is_write_method_flags_mutations() {
    for method in [
        "insertOne",
        "insertMany",
        "updateOne",
        "updateMany",
        "replaceOne",
        "deleteOne",
        "deleteMany",
        "drop",
        "createIndex",
        "dropIndex",
        "renameCollection",
    ] {
        assert!(is_write_method(method), "{} should be a write", method);
    }
}

#[test]
fn is_write_method_allows_reads() {
    for method in [
        "find",
        "findOne",
        "aggregate",
        "countDocuments",
        "distinct",
        "estimatedDocumentCount",
    ] {
        assert!(!is_write_method(method), "{} should be a read", method);
    }
}

#[test]
fn is_write_method_rejects_unknown() {
    assert!(!is_write_method("bogusMethod"));
}
