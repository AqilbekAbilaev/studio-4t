use crate::error::AppError;
use mongodb::Client;
use std::collections::HashMap;
use tokio::sync::Mutex;

/// Holds one MongoDB Client per saved connection, keyed by connection ID.
/// Client is cheap to clone (Arc-backed) and manages its own connection pool internally.
pub struct ConnectionPool {
    clients: Mutex<HashMap<String, Client>>,
}

impl ConnectionPool {
    pub fn new() -> Self {
        Self {
            clients: Mutex::new(HashMap::new()),
        }
    }

    pub async fn get(&self, id: &str) -> Option<Client> {
        self.clients.lock().await.get(id).cloned()
    }

    pub async fn insert(&self, id: String, client: Client) {
        self.clients.lock().await.insert(id, client);
    }

    pub async fn remove(&self, id: &str) {
        self.clients.lock().await.remove(id);
    }

    /// Returns a cached client, or creates and caches a new one from `uri`.
    /// The lock is held only for map reads/writes; network I/O happens outside
    /// the lock so concurrent connections don't block each other.
    pub async fn get_or_create(&self, id: &str, uri: &str) -> Result<Client, AppError> {
        // Fast path: already cached.
        if let Some(client) = self.get(id).await {
            return Ok(client);
        }

        // Slow path: create without holding the lock.
        let client = match Client::with_uri_str(uri).await {
            Ok(val) => val,
            Err(e) => return Err(AppError::Mongo(e)),
        };

        // Re-acquire and insert; another task may have beaten us to it,
        // in which case we prefer the existing client and drop ours.
        let mut map = self.clients.lock().await;
        Ok(map.entry(id.to_string()).or_insert(client).clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The pool itself is a pure data structure — we test its state management
    // logic directly. Real Client construction requires a running MongoDB server
    // so those paths are covered by integration tests instead.

    #[tokio::test]
    async fn get_returns_none_when_empty() {
        let pool = ConnectionPool::new();
        assert!(pool.get("any-id").await.is_none());
    }

    #[tokio::test]
    async fn remove_nonexistent_is_noop() {
        let pool = ConnectionPool::new();
        pool.remove("ghost").await; // must not panic
    }

}
