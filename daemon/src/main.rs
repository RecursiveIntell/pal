mod config;
mod dbus;
mod monitor;
mod nftables;
mod safety;
mod services;

use crate::dbus::monitor::MonitorService;
use crate::dbus::ruleset::RulesetService;
use crate::dbus::services::ServicesService;
use crate::monitor::socket::MonitorSocket;
use crate::nftables::engine::NftEngine;
use crate::safety::audit::AuditLog;
use crate::safety::dead_man::DeadManSwitch;
use crate::safety::snapshots::SnapshotManager;
use crate::services::detector::ServiceDetector;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let engine = Arc::new(NftEngine::new("nft"));
    let snapshots = Arc::new(SnapshotManager::new("/var/lib/palisade/snapshots"));
    let dead_man = Arc::new(DeadManSwitch::new(60));
    let audit = Arc::new(Mutex::new(AuditLog::new("/var/lib/palisade/audit.log")));
    let services = Arc::new(ServiceDetector);
    let monitor_socket = MonitorSocket::new(engine.clone(), "/run/palisade/monitor.sock");
    if let Err(err) = monitor_socket.start().await {
        warn!(error = %err, "failed to start monitor socket on daemon boot");
    }

    let ruleset_service = RulesetService::new(
        engine.clone(),
        snapshots.clone(),
        dead_man.clone(),
        audit.clone(),
        services.clone(),
    );

    let services_iface = ServicesService::new(
        services.clone(),
        engine.clone(),
        snapshots.clone(),
        audit.clone(),
    )?;
    let services_iface_for_watch = services_iface.clone();
    let monitor_iface = MonitorService::new(monitor_socket);

    let conn = zbus::connection::Builder::system()?
        .name("org.palisade.Daemon1")?
        .serve_at("/org/palisade/Daemon1", ruleset_service)?
        .serve_at("/org/palisade/Daemon1", services_iface)?
        .serve_at("/org/palisade/Daemon1", monitor_iface)?
        .build()
        .await?;
    services_iface_for_watch.spawn_name_owner_watcher(conn.clone());

    info!("org.palisade.Daemon1 started");
    tokio::signal::ctrl_c().await?;
    Ok(())
}
