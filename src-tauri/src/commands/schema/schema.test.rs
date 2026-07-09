use super::infer_schema;
use mongodb::bson::doc;

// Fetch a field by path from a report, panicking with a helpful message if it is
// absent, so assertions read clearly.
fn field<'a>(report: &'a super::SchemaReport, path: &str) -> &'a super::FieldSchema {
    match report.fields.iter().find(|f| f.path == path) {
        Some(val) => val,
        None => panic!("expected field {path:?} in schema, got {:?}",
            report.fields.iter().map(|f| f.path.clone()).collect::<Vec<_>>()),
    }
}

#[test]
fn empty_sample_yields_no_fields() {
    let report = infer_schema(&[]);
    assert_eq!(report.sampled, 0);
    assert!(report.fields.is_empty());
}

#[test]
fn counts_presence_across_documents() {
    let docs = vec![
        doc! { "_id": 1, "name": "a" },
        doc! { "_id": 2, "name": "b" },
        doc! { "_id": 3 },
    ];
    let report = infer_schema(&docs);
    assert_eq!(report.sampled, 3);
    // _id is in all three, name in two.
    assert_eq!(field(&report, "_id").present, 3);
    assert_eq!(field(&report, "name").present, 2);
}

#[test]
fn tracks_mixed_types_for_one_path() {
    let docs = vec![
        doc! { "value": 1_i32 },
        doc! { "value": 2_i32 },
        doc! { "value": "three" },
    ];
    let report = infer_schema(&docs);
    let value = field(&report, "value");
    assert_eq!(value.present, 3);
    // int appears twice, string once, and int (higher count) sorts first.
    assert_eq!(value.types.len(), 2);
    assert_eq!(value.types[0].bson_type, "int");
    assert_eq!(value.types[0].count, 2);
    assert_eq!(value.types[1].bson_type, "string");
    assert_eq!(value.types[1].count, 1);
}

#[test]
fn flattens_nested_documents_with_dot_paths() {
    let docs = vec![
        doc! { "address": { "city": "NYC", "zip": "10001" } },
        doc! { "address": { "city": "LA" } },
    ];
    let report = infer_schema(&docs);
    assert_eq!(field(&report, "address").present, 2);
    assert_eq!(field(&report, "address").types[0].bson_type, "object");
    assert_eq!(field(&report, "address.city").present, 2);
    assert_eq!(field(&report, "address.zip").present, 1);
}

#[test]
fn recurses_into_arrays_of_subdocuments() {
    let docs = vec![
        doc! { "items": [ { "sku": "a" }, { "sku": "b" } ] },
        doc! { "items": [ { "sku": "c", "qty": 2_i32 } ] },
    ];
    let report = infer_schema(&docs);
    // The array field itself is present in both docs, typed as array.
    assert_eq!(field(&report, "items").present, 2);
    assert_eq!(field(&report, "items").types[0].bson_type, "array");
    // Element sub-fields flatten under the array path; sku in both docs, qty in one.
    assert_eq!(field(&report, "items.sku").present, 2);
    assert_eq!(field(&report, "items.qty").present, 1);
}

#[test]
fn array_of_subdocs_counts_field_once_per_document() {
    // Two sub-documents in a single document must not double-count coverage.
    let docs = vec![doc! { "items": [ { "sku": "a" }, { "sku": "b" } ] }];
    let report = infer_schema(&docs);
    assert_eq!(field(&report, "items.sku").present, 1);
    assert_eq!(field(&report, "items.sku").types[0].count, 1);
}

#[test]
fn fields_are_sorted_by_path() {
    let docs = vec![doc! { "b": 1_i32, "a": 1_i32, "c": 1_i32 }];
    let report = infer_schema(&docs);
    let paths: Vec<&str> = report.fields.iter().map(|f| f.path.as_str()).collect();
    assert_eq!(paths, vec!["a", "b", "c"]);
}
