// Optional SSH tunnel for a connection: open an SSH session to a bastion, bind a
// local TCP port, and forward each accepted socket to the remote MongoDB host
// through an SSH `direct-tcpip` channel. The MongoDB driver then connects to the
// local port. Pure-Rust (russh), runs on the existing tokio runtime.

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use russh::client;
use russh::keys::{load_secret_key, PrivateKeyWithHashAlg, PublicKey};
use tokio::net::TcpListener;

use crate::error::AppError;

/// How to authenticate to the SSH server.
pub enum SshAuth {
    Password(String),
    Key {
        path: String,
        passphrase: Option<String>,
    },
}

/// Everything needed to open a tunnel: the bastion + auth, and the MongoDB
/// host/port as reachable *from the bastion*.
pub struct SshParams {
    pub ssh_host: String,
    pub ssh_port: u16,
    pub ssh_user: String,
    pub auth: SshAuth,
    pub mongo_host: String,
    pub mongo_port: u16,
}

/// A live tunnel. Dropping it aborts the accept loop and (when the last clone is
/// gone) closes the SSH session.
pub struct SshTunnel {
    pub local_addr: SocketAddr,
    // Keeps the SSH session alive for the channels' lifetime.
    _session: Arc<client::Handle<ClientHandler>>,
    accept_task: tokio::task::JoinHandle<()>,
    // Cleared when the listener or a forward fails — the pool re-establishes a
    // dead tunnel on next use instead of failing forever.
    alive: Arc<AtomicBool>,
}

impl SshTunnel {
    pub fn is_alive(&self) -> bool {
        self.alive.load(Ordering::Relaxed)
    }
}

impl Drop for SshTunnel {
    fn drop(&mut self) {
        self.accept_task.abort();
    }
}

/// Accept-all host-key policy (beta). A known_hosts / trust-on-first-use flow is
/// a planned follow-up.
pub struct ClientHandler;

impl client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(&mut self, _server_public_key: &PublicKey) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

/// Open an SSH session, authenticate, and start forwarding a fresh local port to
/// `mongo_host:mongo_port` through the tunnel.
pub async fn establish(params: SshParams) -> Result<SshTunnel, AppError> {
    let mut config = client::Config::default();
    // Send keepalives so a dropped session is detected instead of hanging.
    config.keepalive_interval = Some(std::time::Duration::from_secs(30));
    let config = Arc::new(config);
    let mut session = match client::connect(
        config,
        (params.ssh_host.as_str(), params.ssh_port),
        ClientHandler,
    )
    .await
    {
        Ok(value) => value,
        Err(e) => return Err(AppError::Ssh(format!("connect failed: {}", e))),
    };

    let auth = match params.auth {
        SshAuth::Password(password) => {
            session
                .authenticate_password(params.ssh_user.clone(), password)
                .await
        }
        SshAuth::Key { path, passphrase } => {
            let key = match load_secret_key(&path, passphrase.as_deref()) {
                Ok(value) => value,
                Err(e) => return Err(AppError::Ssh(format!("could not load key: {}", e))),
            };
            session
                .authenticate_publickey(
                    params.ssh_user.clone(),
                    PrivateKeyWithHashAlg::new(Arc::new(key), None),
                )
                .await
        }
    };
    match auth {
        Ok(result) => {
            if !result.success() {
                return Err(AppError::Ssh(String::from("authentication failed")));
            }
        }
        Err(e) => return Err(AppError::Ssh(format!("authentication error: {}", e))),
    }

    let listener = match TcpListener::bind("127.0.0.1:0").await {
        Ok(value) => value,
        Err(e) => return Err(AppError::Io(e)),
    };
    let local_addr = match listener.local_addr() {
        Ok(value) => value,
        Err(e) => return Err(AppError::Io(e)),
    };

    let session = Arc::new(session);
    let forward_session = Arc::clone(&session);
    let alive = Arc::new(AtomicBool::new(true));
    let accept_alive = Arc::clone(&alive);
    let mongo_host = params.mongo_host;
    let mongo_port = params.mongo_port as u32;
    let local_port = local_addr.port() as u32;

    let accept_task = tokio::spawn(async move {
        loop {
            let (mut socket, _peer) = match listener.accept().await {
                Ok(value) => value,
                Err(_) => {
                    accept_alive.store(false, Ordering::Relaxed);
                    break;
                }
            };
            let session = Arc::clone(&forward_session);
            let forward_alive = Arc::clone(&accept_alive);
            let host = mongo_host.clone();
            tokio::spawn(async move {
                let channel = match session
                    .channel_open_direct_tcpip(host, mongo_port, "127.0.0.1", local_port)
                    .await
                {
                    Ok(value) => value,
                    // A failed channel open usually means the SSH session died.
                    Err(_) => {
                        forward_alive.store(false, Ordering::Relaxed);
                        return;
                    }
                };
                let mut stream = channel.into_stream();
                let _ = tokio::io::copy_bidirectional(&mut socket, &mut stream).await;
            });
        }
    });

    Ok(SshTunnel {
        local_addr: local_addr,
        _session: session,
        accept_task: accept_task,
        alive: alive,
    })
}
