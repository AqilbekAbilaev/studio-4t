use super::diff_docs;
use mongodb::bson::doc;

#[test]
fn identical_collections() {
    let a = vec![doc! { "_id": 1_i32, "v": "x" }, doc! { "_id": 2_i32, "v": "y" }];
    let b = vec![doc! { "_id": 1_i32, "v": "x" }, doc! { "_id": 2_i32, "v": "y" }];
    let r = diff_docs(&a, &b);
    assert_eq!(r.identical_count, 2);
    assert_eq!(r.differing_count, 0);
    assert_eq!(r.only_in_source_count, 0);
    assert_eq!(r.only_in_target_count, 0);
}

#[test]
fn only_in_source_and_target() {
    let a = vec![doc! { "_id": 1_i32 }, doc! { "_id": 2_i32 }];
    let b = vec![doc! { "_id": 2_i32 }, doc! { "_id": 3_i32 }];
    let r = diff_docs(&a, &b);
    assert_eq!(r.only_in_source_count, 1); // id 1
    assert_eq!(r.only_in_target_count, 1); // id 3
    assert_eq!(r.identical_count, 1);      // id 2
    assert_eq!(r.only_in_source[0]["_id"], serde_json::json!(1));
    assert_eq!(r.only_in_target[0]["_id"], serde_json::json!(3));
}

#[test]
fn differing_documents() {
    let a = vec![doc! { "_id": 1_i32, "v": "old" }];
    let b = vec![doc! { "_id": 1_i32, "v": "new" }];
    let r = diff_docs(&a, &b);
    assert_eq!(r.differing_count, 1);
    assert_eq!(r.identical_count, 0);
    assert_eq!(r.differing[0].id, "1");
    assert_eq!(r.differing[0].source["v"], serde_json::json!("old"));
    assert_eq!(r.differing[0].target["v"], serde_json::json!("new"));
}

#[test]
fn field_order_does_not_count_as_difference() {
    let a = vec![doc! { "_id": 1_i32, "a": 1_i32, "b": 2_i32 }];
    let b = vec![doc! { "_id": 1_i32, "b": 2_i32, "a": 1_i32 }];
    let r = diff_docs(&a, &b);
    assert_eq!(r.identical_count, 1);
    assert_eq!(r.differing_count, 0);
}

#[test]
fn id_type_is_significant() {
    // int 1 and string "1" are different keys, so both are "only in" their side.
    let a = vec![doc! { "_id": 1_i32 }];
    let b = vec![doc! { "_id": "1" }];
    let r = diff_docs(&a, &b);
    assert_eq!(r.only_in_source_count, 1);
    assert_eq!(r.only_in_target_count, 1);
    assert_eq!(r.identical_count, 0);
}

#[test]
fn totals_reflect_keyed_counts() {
    let a = vec![doc! { "_id": 1_i32 }, doc! { "_id": 2_i32 }, doc! { "_id": 3_i32 }];
    let b = vec![doc! { "_id": 2_i32 }];
    let r = diff_docs(&a, &b);
    assert_eq!(r.source_total, 3);
    assert_eq!(r.target_total, 1);
}

#[test]
fn empty_collections() {
    let r = diff_docs(&[], &[]);
    assert_eq!(r.source_total, 0);
    assert_eq!(r.target_total, 0);
    assert_eq!(r.identical_count, 0);
}

#[test]
fn nested_content_difference_is_detected() {
    let a = vec![doc! { "_id": 1_i32, "meta": { "x": 1_i32 } }];
    let b = vec![doc! { "_id": 1_i32, "meta": { "x": 2_i32 } }];
    let r = diff_docs(&a, &b);
    assert_eq!(r.differing_count, 1);
}
