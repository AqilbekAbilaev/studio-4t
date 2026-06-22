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

    let has_user = config.username.as_deref().filter(|s| !s.is_empty()).is_some();

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

    let host_part = if config.connection_type == "srv" {
        config.host.clone()
    } else {
        format!("{}:{}", config.host, config.port)
    };

    let mut query: Vec<String> = Vec::new();

    if has_user {
        let auth_db = config.auth_db.as_deref().filter(|s| !s.is_empty()).unwrap_or("admin");
        query.push(format!("authSource={}", auth_db));
    }

    if let Some(rs) = config.replica_set_name.as_deref().filter(|s| !s.is_empty()) {
        query.push(format!("replicaSet={}", rs));
    }

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
    let params = format!(
        "serverSelectionTimeoutMS={TIMEOUT_MS}&connectTimeoutMS={TIMEOUT_MS}"
    );
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
    // Take the first host before any '/' or '?'
    let host_port = match rest.split(|c: char| c == '/' || c == '?').next() {
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
    use crate::storage::ConnectionConfig;

    fn base_config() -> ConnectionConfig {
        ConnectionConfig {
            id: String::from("test"),
            name: String::from("Test"),
            host: String::from("localhost"),
            port: 27017,
            connection_type: String::from("standalone"),
            replica_set_name: None,
            username: None,
            auth_db: None,
            tag: None,
            last_accessed: None,
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
            host: String::from("cluster.example.com"),
            ..base_config()
        };
        assert!(build_uri(&config, None).starts_with("mongodb+srv://"));
        assert!(!build_uri(&config, None).contains(":27017"));
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
    fn extract_stops_at_query_string() {
        assert_eq!(
            extract_host_port("mongodb://localhost:27017?authSource=admin"),
            Some("localhost:27017".into())
        );
    }
}
