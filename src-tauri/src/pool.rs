use crate::error::AppError;
use crate::known_hosts::KnownHostsStore;
use crate::ssh::{self, HostKeyPrompts, SshAuth, SshParams, SshTunnel};
use crate::storage::{ConnectionConfig, SshAuthMethod};
use crate::uri;
use mongodb::Client;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::AppHandle;
use tokio::sync::Mutex;

/// Holds one MongoDB Client per saved connection, keyed by connection ID.
/// Client is cheap to clone (Arc-backed) and manages its own connection pool internally.
/// For SSH-tunnelled connections it also owns the live tunnel, keyed by the same id.
pub struct ConnectionPool {
    clients: Mutex<HashMap<String, Client>>,
    tunnels: Mutex<HashMap<String, Arc<SshTunnel>>>,
    // One lock per connection id, held while a tunnel is established. Without it,
    // two concurrent operations on the same SSH connection each build a tunnel on
    // a different local port; the discarded one's listener is then dropped, and a
    // client cached against that port fails with "connection refused".
    setup_locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
    // Shared with the rest of the app so a tunnel established here verifies the
    // bastion's host key against the same trust store the dialog's Test uses.
    known_hosts: Arc<KnownHostsStore>,
    // The host-key prompt broker + app handle let an SSH handshake started here
    // raise the first-contact trust prompt in the UI (see ssh::ClientHandler).
    prompts: Arc<HostKeyPrompts>,
    app: AppHandle,
}

impl ConnectionPool {
    pub fn new(
        known_hosts: Arc<KnownHostsStore>,
        prompts: Arc<HostKeyPrompts>,
        app: AppHandle,
    ) -> Self {
        Self {
            clients: Mutex::new(HashMap::new()),
            tunnels: Mutex::new(HashMap::new()),
            setup_locks: Mutex::new(HashMap::new()),
            known_hosts: known_hosts,
            prompts: prompts,
            app: app,
        }
    }

    /// The per-connection establishment lock, created on first use.
    async fn setup_lock_for(&self, id: &str) -> Arc<Mutex<()>> {
        let mut locks = self.setup_locks.lock().await;
        Arc::clone(
            locks
                .entry(id.to_string())
                .or_insert_with(|| Arc::new(Mutex::new(()))),
        )
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
    /// connection. Returns the cached client when one exists; only on a cache
    /// miss does it read the password from the keychain and build the URI — so a
    /// hot query path (find/count on an already-open connection) pays no keychain
    /// round-trip. The pool owns credential resolution end to end (it already
    /// reads the SSH secrets in `ensure_tunnel`).
    pub async fn connect(&self, config: &ConnectionConfig) -> Result<Client, AppError> {
        // SSH: ensure the tunnel first (this re-establishes a dead one and evicts
        // the stale client, so the liveness check must run before the cache read).
        if config.ssh_enabled {
            let tunnel = match self.ensure_tunnel(config).await {
                Ok(value) => value,
                Err(e) => return Err(e),
            };
            // Cached client for the live tunnel — no credential lookup needed.
            if let Some(client) = self.get(&config.id).await {
                return Ok(client);
            }
            let password = crate::keychain::get(&config.id);
            let host = String::from("127.0.0.1");
            let port = tunnel.local_addr.port();
            let built_uri = uri::build_uri_to(config, password.as_deref(), &host, port);
            return self
                .get_or_create(&config.id, &uri::with_timeout(&built_uri))
                .await;
        }

        // Non-SSH fast path: a cached client needs no keychain read or URI build.
        if let Some(client) = self.get(&config.id).await {
            return Ok(client);
        }
        let password = crate::keychain::get(&config.id);
        let built_uri = uri::build_uri(config, password.as_deref());
        self.get_or_create(&config.id, &uri::with_timeout(&built_uri))
            .await
    }

    /// Returns the cached SSH tunnel for `config`, establishing one if needed.
    /// SSH secrets are read from the keychain under composite keys.
    async fn ensure_tunnel(&self, config: &ConnectionConfig) -> Result<Arc<SshTunnel>, AppError> {
        // Fast path: a live tunnel already exists — no need to serialize.
        if let Some(existing) = self.tunnels.lock().await.get(&config.id).cloned() {
            if existing.is_alive() {
                return Ok(existing);
            }
        }

        // Serialize establishment per connection so concurrent callers don't each
        // build a tunnel on a different local port (see `setup_locks`).
        let setup_lock = self.setup_lock_for(&config.id).await;
        let _setup_guard = setup_lock.lock().await;

        // Double-check under the lock: another caller may have just established one.
        if let Some(existing) = self.tunnels.lock().await.get(&config.id).cloned() {
            if existing.is_alive() {
                return Ok(existing);
            }
            // Dead tunnel: drop it and the stale client so we re-establish below.
            eprintln!(
                "[ozendb] ssh tunnel for {} is dead; re-establishing",
                config.id
            );
            self.clients.lock().await.remove(&config.id);
            self.tunnels.lock().await.remove(&config.id);
        }

        let auth = match config.ssh_auth_method() {
            SshAuthMethod::Key => SshAuth::Key {
                path: config.ssh_key_file.clone().unwrap_or_default(),
                passphrase: crate::keychain::get(&format!("{}::ssh-key-pass", config.id)),
            },
            SshAuthMethod::Password => SshAuth::Password(
                crate::keychain::get(&format!("{}::ssh-pass", config.id)).unwrap_or_default(),
            ),
        };
        // SSH tunnels forward a single host; multi-host seed lists over SSH are
        // not supported, so the tunnel targets the first host of the list.
        let (mongo_host, mongo_port) = match config.hosts.first() {
            Some(entry) => (entry.host.clone(), entry.port),
            None => (String::from("localhost"), 27017),
        };
        let params = SshParams {
            ssh_host: config.ssh_host.clone().unwrap_or_default(),
            ssh_port: config.ssh_port,
            ssh_user: config.ssh_user.clone().unwrap_or_default(),
            auth: auth,
            mongo_host: mongo_host,
            mongo_port: mongo_port,
        };
        let tunnel = match ssh::establish(
            params,
            Arc::clone(&self.known_hosts),
            Arc::clone(&self.prompts),
            self.app.clone(),
        )
        .await
        {
            Ok(value) => Arc::new(value),
            Err(e) => return Err(e),
        };
        eprintln!(
            "[ozendb] ssh tunnel established for {} on 127.0.0.1:{}",
            config.id,
            tunnel.local_addr.port()
        );
        // A fresh tunnel has a new local port; drop any client cached against the
        // old one so `get_or_create` rebuilds it for this tunnel's port.
        self.clients.lock().await.remove(&config.id);
        self.tunnels
            .lock()
            .await
            .insert(config.id.clone(), Arc::clone(&tunnel));
        Ok(tunnel)
    }
}

// NOTE: the pool previously had two trivial unit tests (empty `get` returns
// None; `remove` of an absent id is a no-op). Constructing a pool now requires a
// real Tauri AppHandle (for the SSH host-key prompt), which a plain unit test
// can't build, and making the pool generic over the runtime purely to keep two
// HashMap-wrapper assertions would be over-engineering. The pool's real paths
// (client creation, tunnels) need a live MongoDB/SSH server and are covered by
// manual/integration testing instead.
