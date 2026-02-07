use crate::monitor::socket::MonitorSocket;

#[derive(Clone)]
pub struct MonitorService {
    monitor_socket: MonitorSocket,
}

impl MonitorService {
    pub fn new(monitor_socket: MonitorSocket) -> Self {
        Self { monitor_socket }
    }
}

#[zbus::interface(name = "org.palisade.Daemon1.Monitor")]
impl MonitorService {
    async fn get_monitor_socket_path(&self) -> zbus::fdo::Result<String> {
        Ok(self.monitor_socket.socket_path())
    }

    async fn start_monitoring(&self) -> zbus::fdo::Result<bool> {
        self.monitor_socket
            .start()
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    async fn stop_monitoring(&self) -> zbus::fdo::Result<bool> {
        self.monitor_socket
            .stop()
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }
}
