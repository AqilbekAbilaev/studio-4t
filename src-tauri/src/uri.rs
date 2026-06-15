use crate::error::AppError;

const TIMEOUT_MS: u64 = 5000;
const TCP_PROBE_SECS: u64 = 3;

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
