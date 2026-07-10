use super::*;

#[test]
fn plain_collection_has_only_create() {
    let command = build_create_command("events", None).unwrap();
    assert_eq!(command.get_str("create").unwrap(), "events");
    assert_eq!(command.len(), 1);
}

#[test]
fn capped_requires_a_size() {
    let options = NewCollectionOptions {
        capped: true,
        size: None,
        max: None,
        time_field: None,
        meta_field: None,
        granularity: None,
        expire_after_seconds: None,
        clustered: false,
        clustered_index_name: None,
    };
    assert!(build_create_command("logs", Some(options)).is_err());
}

#[test]
fn capped_maps_size_and_max() {
    let options = NewCollectionOptions {
        capped: true,
        size: Some(1048576),
        max: Some(1000),
        time_field: None,
        meta_field: None,
        granularity: None,
        expire_after_seconds: None,
        clustered: false,
        clustered_index_name: None,
    };
    let command = build_create_command("logs", Some(options)).unwrap();
    assert_eq!(command.get_bool("capped").unwrap(), true);
    assert_eq!(command.get_i64("size").unwrap(), 1048576);
    assert_eq!(command.get_i64("max").unwrap(), 1000);
}

#[test]
fn nonpositive_max_is_dropped() {
    let options = NewCollectionOptions {
        capped: true,
        size: Some(4096),
        max: Some(0),
        time_field: None,
        meta_field: None,
        granularity: None,
        expire_after_seconds: None,
        clustered: false,
        clustered_index_name: None,
    };
    let command = build_create_command("logs", Some(options)).unwrap();
    assert!(!command.contains_key("max"));
}

#[test]
fn time_series_maps_nested_fields() {
    let options = NewCollectionOptions {
        capped: false,
        size: None,
        max: None,
        time_field: Some("ts".to_string()),
        meta_field: Some("sensor".to_string()),
        granularity: Some("minutes".to_string()),
        expire_after_seconds: Some(86400),
        clustered: false,
        clustered_index_name: None,
    };
    let command = build_create_command("readings", Some(options)).unwrap();
    let timeseries = command.get_document("timeseries").unwrap();
    assert_eq!(timeseries.get_str("timeField").unwrap(), "ts");
    assert_eq!(timeseries.get_str("metaField").unwrap(), "sensor");
    assert_eq!(timeseries.get_str("granularity").unwrap(), "minutes");
    assert_eq!(command.get_i64("expireAfterSeconds").unwrap(), 86400);
}

#[test]
fn blank_time_field_is_rejected() {
    let options = NewCollectionOptions {
        capped: false,
        size: None,
        max: None,
        time_field: Some("   ".to_string()),
        meta_field: None,
        granularity: None,
        expire_after_seconds: None,
        clustered: false,
        clustered_index_name: None,
    };
    assert!(build_create_command("readings", Some(options)).is_err());
}

#[test]
fn empty_optional_time_series_fields_are_omitted() {
    let options = NewCollectionOptions {
        capped: false,
        size: None,
        max: None,
        time_field: Some("ts".to_string()),
        meta_field: Some("".to_string()),
        granularity: Some("".to_string()),
        expire_after_seconds: None,
        clustered: false,
        clustered_index_name: None,
    };
    let command = build_create_command("readings", Some(options)).unwrap();
    let timeseries = command.get_document("timeseries").unwrap();
    assert!(!timeseries.contains_key("metaField"));
    assert!(!timeseries.contains_key("granularity"));
    assert!(!command.contains_key("expireAfterSeconds"));
}

#[test]
fn clustered_maps_fixed_id_index() {
    let options = NewCollectionOptions {
        capped: false,
        size: None,
        max: None,
        time_field: None,
        meta_field: None,
        granularity: None,
        expire_after_seconds: None,
        clustered: true,
        clustered_index_name: Some("primary".to_string()),
    };
    let command = build_create_command("events", Some(options)).unwrap();
    let clustered = command.get_document("clusteredIndex").unwrap();
    assert_eq!(clustered.get_document("key").unwrap().get_i32("_id").unwrap(), 1);
    assert_eq!(clustered.get_bool("unique").unwrap(), true);
    assert_eq!(clustered.get_str("name").unwrap(), "primary");
}

#[test]
fn clustered_without_name_omits_it() {
    let options = NewCollectionOptions {
        capped: false,
        size: None,
        max: None,
        time_field: None,
        meta_field: None,
        granularity: None,
        expire_after_seconds: None,
        clustered: true,
        clustered_index_name: None,
    };
    let command = build_create_command("events", Some(options)).unwrap();
    let clustered = command.get_document("clusteredIndex").unwrap();
    assert!(!clustered.contains_key("name"));
}
