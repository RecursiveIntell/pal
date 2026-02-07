use crate::monitor::counters::CounterPoller;
use crate::nftables::engine::NftEngine;
use std::fs;
use std::io::ErrorKind;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::{oneshot, Mutex};
use tokio::task::JoinHandle;
use tokio::time::{interval, Duration};
use tracing::{debug, error, info};

#[derive(Clone)]
pub struct MonitorSocket {
    inner: Arc<Inner>,
}

struct Inner {
    engine: Arc<NftEngine>,
    socket_path: PathBuf,
    state: Mutex<Option<RunningState>>,
}

struct RunningState {
    shutdown: oneshot::Sender<()>,
    task: JoinHandle<()>,
}

impl MonitorSocket {
    pub fn new(engine: Arc<NftEngine>, socket_path: impl AsRef<Path>) -> Self {
        Self {
            inner: Arc::new(Inner {
                engine,
                socket_path: socket_path.as_ref().to_path_buf(),
                state: Mutex::new(None),
            }),
        }
    }

    pub fn socket_path(&self) -> String {
        self.inner.socket_path.to_string_lossy().to_string()
    }

    pub async fn start(&self) -> anyhow::Result<bool> {
        let mut guard = self.inner.state.lock().await;
        if guard.is_some() {
            return Ok(true);
        }

        let (tx, rx) = oneshot::channel();
        let path = self.inner.socket_path.clone();
        let engine = self.inner.engine.clone();
        let task = tokio::spawn(async move {
            if let Err(err) = run_server(path, engine, rx).await {
                error!(error = %err, "monitor socket server stopped with error");
            }
        });

        *guard = Some(RunningState { shutdown: tx, task });
        Ok(true)
    }

    pub async fn stop(&self) -> anyhow::Result<bool> {
        let mut guard = self.inner.state.lock().await;
        let Some(running) = guard.take() else {
            return Ok(false);
        };

        let _ = running.shutdown.send(());
        let _ = running.task.await;
        Ok(true)
    }
}

async fn run_server(
    socket_path: PathBuf,
    engine: Arc<NftEngine>,
    mut shutdown: oneshot::Receiver<()>,
) -> anyhow::Result<()> {
    if let Some(parent) = socket_path.parent() {
        fs::create_dir_all(parent)?;
    }
    if socket_path.exists() {
        let _ = fs::remove_file(&socket_path);
    }

    let listener = UnixListener::bind(&socket_path)?;
    fs::set_permissions(&socket_path, fs::Permissions::from_mode(0o666))?;
    info!(path = %socket_path.to_string_lossy(), "monitor socket started");

    let mut clients: Vec<UnixStream> = Vec::new();
    let mut ticker = interval(Duration::from_secs(1));
    let mut poller = CounterPoller::default();

    loop {
        tokio::select! {
            _ = &mut shutdown => {
                break;
            }
            accept_result = listener.accept() => {
                match accept_result {
                    Ok((stream, _)) => clients.push(stream),
                    Err(err) => error!(error=%err, "monitor accept failed"),
                }
            }
            _ = ticker.tick() => {
                let summary = match poller.poll(engine.clone()).await {
                    Ok(summary) => summary,
                    Err(err) => {
                        error!(error=%err, "counter polling failed");
                        continue;
                    }
                };

                let payload = match rmp_serde::to_vec_named(&summary) {
                    Ok(payload) => payload,
                    Err(err) => {
                        error!(error=%err, "monitor serialization failed");
                        continue;
                    }
                };

                let Ok(len_u32) = u32::try_from(payload.len()) else {
                    error!("monitor payload too large");
                    continue;
                };

                let mut frame = Vec::with_capacity(4 + payload.len());
                frame.extend_from_slice(&len_u32.to_be_bytes());
                frame.extend_from_slice(&payload);

                let mut alive = Vec::with_capacity(clients.len());
                for mut client in clients.drain(..) {
                    match client.write_all(&frame).await {
                        Ok(()) => alive.push(client),
                        Err(err) => {
                            if err.kind() == ErrorKind::BrokenPipe
                                || err.kind() == ErrorKind::ConnectionReset
                            {
                                debug!(error=%err, "monitor client disconnected");
                            } else {
                                error!(error=%err, "monitor client write failed");
                            }
                        }
                    }
                }
                clients = alive;
            }
        }
    }

    drop(listener);
    let _ = fs::remove_file(&socket_path);
    info!(path = %socket_path.to_string_lossy(), "monitor socket stopped");
    Ok(())
}
