use std::sync::OnceLock;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::net::UnixStream;
use tokio::sync::Mutex;
use tokio::time::timeout;
use zbus::Connection;

struct MonitorClient {
    path: String,
    stream: UnixStream,
}

static MONITOR_CLIENT: OnceLock<Mutex<Option<MonitorClient>>> = OnceLock::new();

fn monitor_client() -> &'static Mutex<Option<MonitorClient>> {
    MONITOR_CLIENT.get_or_init(|| Mutex::new(None))
}

async fn monitor_proxy(connection: &Connection) -> Result<zbus::Proxy<'_>, String> {
    zbus::Proxy::new(
        connection,
        "org.palisade.Daemon1",
        "/org/palisade/Daemon1",
        "org.palisade.Daemon1.Monitor",
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_monitor_socket_path() -> Result<String, String> {
    let conn = Connection::system().await.map_err(|e| e.to_string())?;
    let proxy = monitor_proxy(&conn).await?;
    proxy
        .call("GetMonitorSocketPath", &())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_monitoring() -> Result<bool, String> {
    let conn = Connection::system().await.map_err(|e| e.to_string())?;
    let proxy = monitor_proxy(&conn).await?;
    proxy
        .call("StartMonitoring", &())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop_monitoring() -> Result<bool, String> {
    let conn = Connection::system().await.map_err(|e| e.to_string())?;
    let proxy = monitor_proxy(&conn).await?;
    let stopped: bool = proxy
        .call("StopMonitoring", &())
        .await
        .map_err(|e| e.to_string())?;
    *monitor_client().lock().await = None;
    Ok(stopped)
}

#[tauri::command]
pub async fn read_monitor_sample(socket_path: String) -> Result<String, String> {
    let read_future = async {
        let mut guard = monitor_client().lock().await;
        if guard
            .as_ref()
            .map(|client| client.path != socket_path)
            .unwrap_or(true)
        {
            *guard = Some(MonitorClient {
                path: socket_path.clone(),
                stream: connect_monitor_stream(&socket_path).await?,
            });
        }

        for attempt in 0..=1 {
            let Some(client) = guard.as_mut() else {
                return Err("monitor client unavailable".to_string());
            };

            match read_monitor_frame(&mut client.stream).await {
                Ok(frame) => return Ok(frame),
                Err(err) if attempt == 0 => {
                    client.stream = connect_monitor_stream(&socket_path).await?;
                    let _ = err;
                }
                Err(err) => {
                    *guard = None;
                    return Err(err);
                }
            }
        }

        Err("monitor read failed".to_string())
    };

    timeout(Duration::from_secs(3), read_future)
        .await
        .map_err(|_| "monitor read timed out".to_string())?
}

async fn connect_monitor_stream(path: &str) -> Result<UnixStream, String> {
    UnixStream::connect(path).await.map_err(|e| e.to_string())
}

async fn read_monitor_frame(stream: &mut UnixStream) -> Result<String, String> {
    let mut header = [0_u8; 4];
    stream
        .read_exact(&mut header)
        .await
        .map_err(|e| e.to_string())?;

    let len = u32::from_be_bytes(header);
    let payload_len = usize::try_from(len).map_err(|e| e.to_string())?;
    let mut payload = vec![0_u8; payload_len];
    stream
        .read_exact(&mut payload)
        .await
        .map_err(|e| e.to_string())?;

    let summary: palisade_shared::TrafficSummary =
        rmp_serde::from_slice(&payload).map_err(|e| e.to_string())?;
    serde_json::to_string(&summary).map_err(|e| e.to_string())
}
