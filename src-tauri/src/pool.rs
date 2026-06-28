use crate::error::AppError;
use crate::ssh::{self, SshAuth, SshParams, SshTunnel};
use crate::storage::ConnectionConfig;
use crate::uri;
use mongodb::Client;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Holds one MongoDB Client per saved connection, keyed by connection ID.
/// Client is cheap to clone (Arc-backed) and manages its own connection pool internally.
/// For SSH-tunnelled connections it also owns the live tunnel, keyed by the same id.
pub struct ConnectionPool {
    clients: Mutex<HashMap<String, Client>>,
    tunnels: Mutex<HashMap<String, Arc<SshTunnel>>>,
}

impl ConnectionPool {
    pub fn new() -> Self {
        Self {
            clients: Mutex::new(HashMap::new()),
            tunnels: Mutex::new(HashMap::new()),
        }
    }

    pub async fn get(&self, id: &str) -> Option<Client> {
        self.clients.lock().await.get(id).cloned()
    }

    pub async fn remove(&self, id: &str) {
        self.clients.lock().await.remove(id);
        // Dropping the last Arc tears the tunnel (accept loop + session) down.
        self.tunnels.lock().await.remove(id);
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

    /// Single entry point every command uses to obtain a client for a
    /// connection: builds the URI from the stored config (+ keychain password)
    /// and returns a cached-or-new client keyed by the connection id.
    pub async fn connect(
        &self,
        config: &ConnectionConfig,
        password: Option<&str>,
    ) -> Result<Client, AppError> {
        // SSH: ensure the tunnel and point the driver at the local forwarded port.
        if config.ssh_enabled {
            let tunnel = match self.ensure_tunnel(config).await {
                Ok(value) => value,
                Err(e) => return Err(e),
            };
            let host = String::from("127.0.0.1");
            let port = tunnel.local_addr.port();
            let built_uri = uri::build_uri_to(config, password, &host, port);
            return self
                .get_or_create(&config.id, &uri::with_timeout(&built_uri))
                .await;
        }

        let built_uri = uri::build_uri(config, password);
        self.get_or_create(&config.id, &uri::with_timeout(&built_uri))
            .await
    }

    /// Returns the cached SSH tunnel for `config`, establishing one if needed.
    /// SSH secrets are read from the keychain under composite keys.
    async fn ensure_tunnel(&self, config: &ConnectionConfig) -> Result<Arc<SshTunnel>, AppError> {
        if let Some(existing) = self.tunnels.lock().await.get(&config.id).cloned() {
            if existing.is_alive() {
                return Ok(existing);
            }
            // Dead tunnel: drop it and the stale client so we re-establish below.
            self.clients.lock().await.remove(&config.id);
            self.tunnels.lock().await.remove(&config.id);
        }

        let auth = match config.ssh_auth.as_deref() {
            Some("key") => SshAuth::Key {
                path: config.ssh_key_file.clone().unwrap_or_default(),
                passphrase: crate::keychain::get(&format!("{}::ssh-key-pass", config.id)),
            },
            _ => SshAuth::Password(
                crate::keychain::get(&format!("{}::ssh-pass", config.id)).unwrap_or_default(),
            ),
        };
        let params = SshParams {
            ssh_host: config.ssh_host.clone().unwrap_or_default(),
            ssh_port: config.ssh_port,
            ssh_user: config.ssh_user.clone().unwrap_or_default(),
            auth: auth,
            mongo_host: config.host.clone(),
            mongo_port: config.port,
        };
        let tunnel = match ssh::establish(params).await {
            Ok(value) => Arc::new(value),
            Err(e) => return Err(e),
        };
        self.tunnels
            .lock()
            .await
            .insert(config.id.clone(), Arc::clone(&tunnel));
        Ok(tunnel)
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
