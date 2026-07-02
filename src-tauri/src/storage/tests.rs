use super::*;
use tempfile::tempdir;

fn conn(id: &str, name: &str) -> ConnectionConfig {
    ConnectionConfig {
        id: id.into(),
        name: name.into(),
        hosts: vec![HostEntry { host: String::from("localhost"), port: 27017 }],
        connection_type: String::from("standalone"),
        replica_set_name: None,
        username: None,
        auth_db: None,
        auth_mechanism: None,
        options: BTreeMap::new(),
        tls: false,
        tls_ca_file: None,
        tls_cert_key_file: None,
        tls_allow_invalid_certificates: false,
        ssh_enabled: false,
        ssh_host: None,
        ssh_port: 22,
        ssh_user: None,
        ssh_auth: None,
        ssh_key_file: None,
        tag: None,
        folder_id: None,
        last_accessed: None,
        open: false,
    }
}

fn storage_in_tempdir() -> (Storage, tempfile::TempDir) {
    let dir = tempdir().unwrap();
    let s = Storage::new(dir.path().join("connections.json"));
    (s, dir)
}

#[test]
fn load_returns_empty_when_file_missing() {
    let (storage, _dir) = storage_in_tempdir();
    assert!(storage.load().is_empty());
}

#[test]
fn save_and_load_roundtrip() {
    let (storage, _dir) = storage_in_tempdir();
    let conns = vec![
        conn("1", "Local"),
        conn("2", "Prod"),
    ];
    storage.save(&conns).unwrap();
    assert_eq!(storage.load(), conns);
}

#[test]
fn save_overwrites_existing_file() {
    let (storage, _dir) = storage_in_tempdir();
    storage.save(&[conn("1", "Old")]).unwrap();
    storage.save(&[conn("1", "New")]).unwrap();
    let loaded = storage.load();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].name, "New");
}

#[test]
fn load_returns_empty_on_corrupt_file() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("connections.json");
    std::fs::write(&path, "not valid json {{ garbage").unwrap();
    let storage = Storage::new(path);
    assert!(storage.load().is_empty());
}

#[test]
fn add_appends_to_existing() {
    let (storage, _dir) = storage_in_tempdir();
    storage.add(conn("1", "Local")).unwrap();
    storage.add(conn("2", "Prod")).unwrap();
    let loaded = storage.load();
    assert_eq!(loaded.len(), 2);
    assert_eq!(loaded[1].id, "2");
}

#[test]
fn remove_deletes_by_id() {
    let (storage, _dir) = storage_in_tempdir();
    storage.save(&[
        conn("1", "Keep"),
        conn("2", "Delete"),
    ]).unwrap();
    storage.remove("2").unwrap();
    let loaded = storage.load();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].id, "1");
}

#[test]
fn remove_nonexistent_id_is_noop() {
    let (storage, _dir) = storage_in_tempdir();
    storage.save(&[conn("1", "Local")]).unwrap();
    storage.remove("999").unwrap();
    assert_eq!(storage.load().len(), 1);
}

#[test]
fn load_migrates_legacy_single_host() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("connections.json");
    // A pre-multi-host record: top-level host/port, no `hosts` array.
    std::fs::write(
        &path,
        r#"[{"id":"1","name":"Old","host":"db.example.com","port":27018,"connection_type":"standalone"}]"#,
    ).unwrap();
    let storage = Storage::new(path);
    let loaded = storage.load();
    assert_eq!(loaded.len(), 1);
    assert_eq!(
        loaded[0].hosts,
        vec![HostEntry { host: String::from("db.example.com"), port: 27018 }]
    );
}

#[test]
fn load_migrates_legacy_dns_type_to_srv() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("connections.json");
    std::fs::write(
        &path,
        r#"[{"id":"1","name":"Atlas","host":"cluster.example.com","port":27017,"connection_type":"dns"}]"#,
    ).unwrap();
    let storage = Storage::new(path);
    let loaded = storage.load();
    assert_eq!(loaded[0].connection_type, "srv");
}

#[test]
fn load_preserves_existing_hosts_array() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("connections.json");
    std::fs::write(
        &path,
        r#"[{"id":"1","name":"RS","hosts":[{"host":"a","port":1},{"host":"b","port":2}],"connection_type":"replica"}]"#,
    ).unwrap();
    let storage = Storage::new(path);
    let loaded = storage.load();
    assert_eq!(loaded[0].hosts.len(), 2);
    assert_eq!(loaded[0].hosts[1].host, "b");
}

#[test]
fn save_creates_parent_directories() {
    let dir = tempdir().unwrap();
    let nested_path = dir.path().join("a").join("b").join("connections.json");
    let storage = Storage::new(nested_path);
    storage.save(&[conn("1", "Local")]).unwrap();
    assert_eq!(storage.load().len(), 1);
}
