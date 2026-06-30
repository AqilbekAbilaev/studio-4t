use crate::error::AppError;
use crate::storage::ConnectionConfig;

const TIMEOUT_MS: u64 = 5000;
const TCP_PROBE_SECS: u64 = 3;

/// Percent-encodes a string per RFC 3986, encoding every byte that is not
/// an unreserved character. Handles non-ASCII by encoding each UTF-8 byte.
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9'
            | b'-' | b'_' | b'.' | b'~' => out.push(byte as char),
            b => out.push_str(&format!("%{:02X}", b)),
        }
    }
    out
}

/// Builds a MongoDB connection URI from a stored `ConnectionConfig` plus the
/// password fetched separately from the OS keychain.
/// The resulting URI is suitable for passing to `with_timeout()` and then
/// to `Client::with_uri_str()`.
pub fn build_uri(config: &ConnectionConfig, password: Option<&str>) -> String {
    let scheme = if config.connection_type == "srv" {
        "mongodb+srv"
    } else {
        "mongodb"
    };

    let (creds, has_user) = build_credentials(config, password);

    let host_part = if config.connection_type == "srv" {
        // SRV uses a single hostname and no port; take the first host.
        match config.hosts.first() {
            Some(entry) => entry.host.clone(),
            None => String::from("localhost"),
        }
    } else if config.hosts.is_empty() {
        String::from("localhost:27017")
    } else {
        config
            .hosts
            .iter()
            .map(|entry| format!("{}:{}", entry.host, entry.port))
            .collect::<Vec<String>>()
            .join(",")
    };

    let mut query: Vec<String> = Vec::new();
    push_auth_query(config, has_user, &mut query);

    if let Some(rs) = config.replica_set_name.as_deref().filter(|s| !s.is_empty()) {
        query.push(format!("replicaSet={}", rs));
    }

    push_tls_query(config, &mut query);
    push_options(config, &mut query);

    assemble(scheme, &creds, &host_part, &query)
}

/// Like `build_uri` but targets an explicit `host:port` (used for an SSH tunnel's
/// local forwarded port). Forces a standard `mongodb://` direct connection — no
/// SRV, no replica-set discovery (which would bypass the tunnel) — while keeping
/// the same credentials, auth, and TLS options.
pub fn build_uri_to(
    config: &ConnectionConfig,
    password: Option<&str>,
    host: &str,
    port: u16,
) -> String {
    let (creds, has_user) = build_credentials(config, password);
    let host_part = format!("{}:{}", host, port);

    let mut query: Vec<String> = Vec::new();
    push_auth_query(config, has_user, &mut query);
    query.push(String::from("directConnection=true"));
    push_tls_query(config, &mut query);
    push_options(config, &mut query);

    assemble("mongodb", &creds, &host_part, &query)
}

/// Appends the passthrough `options` (any driver parameter the dedicated fields
/// don't model) to the query string verbatim. Keys are emitted in sorted order
/// (`BTreeMap`), so the built URI is deterministic.
fn push_options(config: &ConnectionConfig, query: &mut Vec<String>) {
    for (key, value) in config.options.iter() {
        if key.is_empty() {
            continue;
        }
        query.push(format!("{}={}", key, value));
    }
}

/// Returns the `user:pass@` prefix (empty when no auth) and whether a user is set.
fn build_credentials(config: &ConnectionConfig, password: Option<&str>) -> (String, bool) {
    // "none" auth mechanism means no credentials in the URI at all.
    let is_no_auth = config.auth_mechanism.as_deref() == Some("none");
    let has_user = !is_no_auth
        && config.username.as_deref().filter(|s| !s.is_empty()).is_some();

    let creds = if has_user {
        let u = percent_encode(config.username.as_deref().unwrap_or(""));
        let p = match password.filter(|s| !s.is_empty()) {
            Some(pw) => format!(":{}", percent_encode(pw)),
            None => String::new(),
        };
        format!("{u}{p}@")
    } else {
        String::new()
    };
    (creds, has_user)
}

fn push_auth_query(config: &ConnectionConfig, has_user: bool, query: &mut Vec<String>) {
    if has_user {
        let auth_db = config.auth_db.as_deref().filter(|s| !s.is_empty()).unwrap_or("admin");
        query.push(format!("authSource={}", auth_db));
    }
    // Explicit mechanism for any mode other than "none" and the implicit default
    // (None / empty = let the driver negotiate).
    if let Some(mech) = config.auth_mechanism.as_deref().filter(|s| !s.is_empty() && *s != "none") {
        query.push(format!("authMechanism={}", mech));
    }
}

fn push_tls_query(config: &ConnectionConfig, query: &mut Vec<String>) {
    // File paths are percent-encoded; the driver decodes query values.
    if config.tls {
        query.push(String::from("tls=true"));
        if let Some(ca) = config.tls_ca_file.as_deref().filter(|s| !s.is_empty()) {
            query.push(format!("tlsCAFile={}", percent_encode(ca)));
        }
        if let Some(cert) = config.tls_cert_key_file.as_deref().filter(|s| !s.is_empty()) {
            query.push(format!("tlsCertificateKeyFile={}", percent_encode(cert)));
        }
        if config.tls_allow_invalid_certificates {
            query.push(String::from("tlsAllowInvalidCertificates=true"));
        }
    }
}

fn assemble(scheme: &str, creds: &str, host_part: &str, query: &[String]) -> String {
    if query.is_empty() {
        format!("{scheme}://{creds}{host_part}/")
    } else {
        format!("{scheme}://{creds}{host_part}/?{}", query.join("&"))
    }
}

/// Appends MongoDB server-selection and connect timeout query parameters.
/// Handles URIs with and without an existing database path or query string.
///
/// MongoDB requires a '/' before '?', so:
///   mongodb://host:27017          → mongodb://host:27017/?params
///   mongodb://host:27017/db       → mongodb://host:27017/db?params
///   mongodb://host:27017/?foo=1   → mongodb://host:27017/?foo=1&params
pub fn with_timeout(uri: &str) -> String {
    // Only supply our defaults for timeouts the URI doesn't already set, so an
    // explicit value carried over from a pasted URI (or the Advanced tab) wins.
    let mut additions: Vec<String> = Vec::new();
    if !uri.contains("serverSelectionTimeoutMS") {
        additions.push(format!("serverSelectionTimeoutMS={TIMEOUT_MS}"));
    }
    if !uri.contains("connectTimeoutMS") {
        additions.push(format!("connectTimeoutMS={TIMEOUT_MS}"));
    }
    if additions.is_empty() {
        return uri.to_string();
    }
    let params = additions.join("&");
    if uri.contains('?') {
        format!("{uri}&{params}")
    } else {
        let scheme_end = uri.find("://").map(|i| i + 3).unwrap_or(0);
        let has_path = uri[scheme_end..].contains('/');
        if has_path {
            format!("{uri}?{params}")
        } else {
            format!("{uri}/?{params}")
        }
    }
}

/// Extracts "host:port" from a standard `mongodb://` URI for the TCP probe.
/// Returns `None` for `mongodb+srv://` or URIs we cannot parse safely.
pub fn extract_host_port(uri: &str) -> Option<String> {
    let rest = match uri.strip_prefix("mongodb://") {
        Some(val) => val,
        None => return None,
    };
    // Drop credentials (user:pass@host → host)
    let rest = rest.find('@').map_or(rest, |i| &rest[i + 1..]);
    // Take the host section before any '/' or '?'
    let hosts = match rest.split(|c: char| c == '/' || c == '?').next() {
        Some(val) => val,
        None => return None,
    };
    // A multi-host seed list — probe just the first host.
    let host_port = match hosts.split(',').next() {
        Some(val) => val,
        None => return None,
    };
    if host_port.is_empty() {
        return None;
    }
    if host_port.contains(':') {
        Some(host_port.to_string())
    } else {
        Some(format!("{host_port}:27017"))
    }
}

/// Performs an async TCP probe against the first host in the URI.
/// Returns immediately on ECONNREFUSED (port closed); times out after 3 s
/// for unreachable hosts (firewall, wrong IP, etc.).
/// No-op for `mongodb+srv://` or URIs we cannot parse.
pub async fn tcp_probe(uri: &str) -> Result<(), AppError> {
    let host_port = match extract_host_port(uri) {
        Some(val) => val,
        None => return Ok(()),
    };

    let addrs: Vec<_> = match tokio::net::lookup_host(&host_port).await {
        Ok(val) => val.collect(),
        Err(e) => return Err(AppError::Unreachable {
            address: host_port.clone(),
            reason: e.to_string(),
        }),
    };

    if addrs.is_empty() {
        return Ok(());
    }

    // Try every resolved address (e.g. ::1 and 127.0.0.1 for "localhost").
    // Succeed as soon as any one connects; only fail if all are refused/timeout.
    let mut last_err = String::new();
    for addr in &addrs {
        match tokio::time::timeout(
            std::time::Duration::from_secs(TCP_PROBE_SECS),
            tokio::net::TcpStream::connect(addr),
        )
        .await
        {
            Ok(Ok(_)) => return Ok(()),
            Ok(Err(e)) => last_err = e.to_string(),
            Err(_) => last_err = format!("timed out connecting to {addr}"),
        }
    }

    Err(AppError::Unreachable {
        address: host_port,
        reason: last_err,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{ConnectionConfig, HostEntry};

    fn base_config() -> ConnectionConfig {
        ConnectionConfig {
            id: String::from("test"),
            name: String::from("Test"),
            hosts: vec![HostEntry { host: String::from("localhost"), port: 27017 }],
            connection_type: String::from("standalone"),
            replica_set_name: None,
            username: None,
            auth_db: None,
            auth_mechanism: None,
            options: std::collections::BTreeMap::new(),
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
            last_accessed: None,
            open: false,
        }
    }

    #[test]
    fn build_uri_no_auth() {
        let config = base_config();
        assert_eq!(build_uri(&config, None), "mongodb://localhost:27017/");
    }

    #[test]
    fn build_uri_with_auth() {
        let config = ConnectionConfig {
            username: Some(String::from("alice")),
            auth_db: Some(String::from("admin")),
            ..base_config()
        };
        assert_eq!(
            build_uri(&config, Some("secret")),
            "mongodb://alice:secret@localhost:27017/?authSource=admin"
        );
    }

    #[test]
    fn build_uri_encodes_special_chars_in_credentials() {
        let config = ConnectionConfig {
            username: Some(String::from("user@example")),
            auth_db: Some(String::from("admin")),
            ..base_config()
        };
        let uri = build_uri(&config, Some("p@ss:word"));
        assert!(uri.contains("user%40example"));
        assert!(uri.contains("p%40ss%3Aword"));
    }

    #[test]
    fn build_uri_srv_scheme() {
        let config = ConnectionConfig {
            connection_type: String::from("srv"),
            hosts: vec![HostEntry { host: String::from("cluster.example.com"), port: 27017 }],
            ..base_config()
        };
        assert!(build_uri(&config, None).starts_with("mongodb+srv://"));
        assert!(!build_uri(&config, None).contains(":27017"));
    }

    #[test]
    fn build_uri_multi_host_seed_list() {
        let config = ConnectionConfig {
            connection_type: String::from("replica"),
            replica_set_name: Some(String::from("rs0")),
            hosts: vec![
                HostEntry { host: String::from("a.example.com"), port: 27017 },
                HostEntry { host: String::from("b.example.com"), port: 27017 },
                HostEntry { host: String::from("c.example.com"), port: 27018 },
            ],
            ..base_config()
        };
        let uri = build_uri(&config, None);
        assert!(uri.contains("a.example.com:27017,b.example.com:27017,c.example.com:27018"));
        assert!(uri.contains("replicaSet=rs0"));
    }

    #[test]
    fn build_uri_empty_hosts_falls_back() {
        let config = ConnectionConfig { hosts: vec![], ..base_config() };
        assert_eq!(build_uri(&config, None), "mongodb://localhost:27017/");
    }

    #[test]
    fn build_uri_appends_passthrough_options() {
        let mut options = std::collections::BTreeMap::new();
        options.insert(String::from("retryWrites"), String::from("true"));
        options.insert(String::from("socketTimeoutMS"), String::from("600000"));
        let config = ConnectionConfig { options: options, ..base_config() };
        let uri = build_uri(&config, None);
        assert!(uri.contains("retryWrites=true"));
        assert!(uri.contains("socketTimeoutMS=600000"));
    }

    #[test]
    fn build_uri_tls_with_files() {
        let config = ConnectionConfig {
            tls: true,
            tls_ca_file: Some(String::from("/etc/ssl/My CA.pem")),
            tls_cert_key_file: Some(String::from("/etc/ssl/client.pem")),
            tls_allow_invalid_certificates: true,
            ..base_config()
        };
        let uri = build_uri(&config, None);
        assert!(uri.contains("tls=true"));
        // path is percent-encoded ('/' → %2F, ' ' → %20)
        assert!(uri.contains("tlsCAFile=%2Fetc%2Fssl%2FMy%20CA.pem"));
        assert!(uri.contains("tlsCertificateKeyFile=%2Fetc%2Fssl%2Fclient.pem"));
        assert!(uri.contains("tlsAllowInvalidCertificates=true"));
    }

    #[test]
    fn build_uri_no_tls_omits_params() {
        let uri = build_uri(&base_config(), None);
        assert!(!uri.contains("tls"));
    }

    #[test]
    fn build_uri_replica_set() {
        let config = ConnectionConfig {
            connection_type: String::from("replica"),
            replica_set_name: Some(String::from("rs0")),
            ..base_config()
        };
        assert!(build_uri(&config, None).contains("replicaSet=rs0"));
    }

    #[test]
    fn build_uri_username_no_password() {
        let config = ConnectionConfig {
            username: Some(String::from("alice")),
            auth_db: Some(String::from("admin")),
            ..base_config()
        };
        let uri = build_uri(&config, None);
        assert!(uri.starts_with("mongodb://alice@"));
        assert!(uri.contains("authSource=admin"));
    }

    #[test]
    fn build_uri_auth_mechanism_scram_sha_256() {
        let config = ConnectionConfig {
            username: Some(String::from("alice")),
            auth_db: Some(String::from("admin")),
            auth_mechanism: Some(String::from("SCRAM-SHA-256")),
            ..base_config()
        };
        let uri = build_uri(&config, Some("secret"));
        assert!(uri.contains("authMechanism=SCRAM-SHA-256"));
    }

    #[test]
    fn build_uri_auth_mechanism_scram_sha_1() {
        let config = ConnectionConfig {
            username: Some(String::from("alice")),
            auth_db: Some(String::from("admin")),
            auth_mechanism: Some(String::from("SCRAM-SHA-1")),
            ..base_config()
        };
        let uri = build_uri(&config, Some("secret"));
        assert!(uri.contains("authMechanism=SCRAM-SHA-1"));
    }

    #[test]
    fn build_uri_auth_mechanism_plain() {
        let config = ConnectionConfig {
            username: Some(String::from("alice")),
            auth_db: Some(String::from("$external")),
            auth_mechanism: Some(String::from("PLAIN")),
            ..base_config()
        };
        let uri = build_uri(&config, Some("secret"));
        assert!(uri.contains("authMechanism=PLAIN"));
    }

    #[test]
    fn build_uri_auth_mechanism_none_omits_credentials() {
        let config = ConnectionConfig {
            username: Some(String::from("alice")),
            auth_mechanism: Some(String::from("none")),
            ..base_config()
        };
        let uri = build_uri(&config, Some("secret"));
        assert!(!uri.contains("alice"));
        assert!(!uri.contains("secret"));
        assert!(!uri.contains("authMechanism"));
    }

    // --- with_timeout ---

    #[test]
    fn with_timeout_adds_slash_when_no_path() {
        let result = with_timeout("mongodb://localhost:27017");
        assert!(result.contains("/?"));
        assert!(result.contains("serverSelectionTimeoutMS=5000"));
        assert!(result.contains("connectTimeoutMS=5000"));
    }

    #[test]
    fn with_timeout_preserves_existing_path() {
        let result = with_timeout("mongodb://localhost:27017/mydb");
        assert!(result.starts_with("mongodb://localhost:27017/mydb?"));
    }

    #[test]
    fn with_timeout_appends_to_existing_query() {
        let result = with_timeout("mongodb://localhost:27017/?authSource=admin");
        assert!(result.contains("authSource=admin"));
        assert!(result.contains("serverSelectionTimeoutMS=5000"));
        // Should use & not ?
        assert!(result.contains("admin&server"));
    }

    #[test]
    fn with_timeout_works_with_credentials() {
        let result = with_timeout("mongodb://user:pass@host:27017");
        assert!(result.contains("host:27017/?"));
    }

    #[test]
    fn with_timeout_does_not_override_explicit_values() {
        // An explicit connectTimeoutMS from the URI must not be duplicated.
        let result = with_timeout("mongodb://localhost:27017/?connectTimeoutMS=10000");
        assert!(result.contains("connectTimeoutMS=10000"));
        assert!(!result.contains("connectTimeoutMS=5000"));
        // The unset one still gets the default.
        assert!(result.contains("serverSelectionTimeoutMS=5000"));
    }

    #[test]
    fn with_timeout_works_with_srv_scheme() {
        let result = with_timeout("mongodb+srv://cluster.example.com");
        // SRV URIs have no port path, still gets params appended
        assert!(result.contains("serverSelectionTimeoutMS=5000"));
    }

    // --- extract_host_port ---

    #[test]
    fn extract_basic() {
        assert_eq!(
            extract_host_port("mongodb://localhost:27017"),
            Some("localhost:27017".into())
        );
    }

    #[test]
    fn extract_with_database_path() {
        assert_eq!(
            extract_host_port("mongodb://localhost:27017/mydb"),
            Some("localhost:27017".into())
        );
    }

    #[test]
    fn extract_with_credentials() {
        assert_eq!(
            extract_host_port("mongodb://user:pass@prod.example.com:27017"),
            Some("prod.example.com:27017".into())
        );
    }

    #[test]
    fn extract_adds_default_port_when_missing() {
        assert_eq!(
            extract_host_port("mongodb://localhost"),
            Some("localhost:27017".into())
        );
    }

    #[test]
    fn extract_returns_none_for_srv() {
        assert_eq!(extract_host_port("mongodb+srv://cluster.example.com"), None);
    }

    #[test]
    fn extract_returns_none_for_empty_host() {
        assert_eq!(extract_host_port("mongodb://"), None);
    }

    #[test]
    fn extract_takes_first_host_of_seed_list() {
        assert_eq!(
            extract_host_port("mongodb://a.example.com:27017,b.example.com:27017/admin"),
            Some("a.example.com:27017".into())
        );
    }

    #[test]
    fn extract_stops_at_query_string() {
        assert_eq!(
            extract_host_port("mongodb://localhost:27017?authSource=admin"),
            Some("localhost:27017".into())
        );
    }
}
