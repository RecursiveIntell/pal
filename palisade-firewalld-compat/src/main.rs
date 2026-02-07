mod interface;
mod rich_rule;
mod service_db;
mod zone_map;

use crate::interface::{PortSpec, RuntimeState, SharedState};
use crate::rich_rule::parse_rich_rule_to_expr;
use crate::service_db::ServiceDatabase;
use crate::zone_map::default_zones;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};
use zbus::{Connection, Proxy};

#[derive(Clone)]
struct ZoneInterface {
    state: SharedState,
    service_db: Arc<ServiceDatabase>,
    connection: Connection,
}

#[derive(Clone)]
struct RootInterface {
    state: SharedState,
}

impl ZoneInterface {
    async fn services_proxy(&self) -> Result<Proxy<'_>, String> {
        Proxy::new(
            &self.connection,
            "org.palisade.Daemon1",
            "/org/palisade/Daemon1",
            "org.palisade.Daemon1.Services",
        )
        .await
        .map_err(|e| e.to_string())
    }

    async fn register_port_rule(&self, protocol: &str, port: &str) -> Result<String, String> {
        let proxy = self.services_proxy().await?;
        if let Some((start, end)) = parse_port_range(port) {
            if start == end {
                let (rule_id, error): (String, String) = proxy
                    .call(
                        "RegisterServicePort",
                        &(
                            "firewalld-compat".to_string(),
                            protocol.to_string(),
                            start,
                            false,
                        ),
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                if error.is_empty() {
                    Ok(rule_id)
                } else {
                    Err(error)
                }
            } else {
                let (rule_id, error): (String, String) = proxy
                    .call(
                        "RegisterServicePortRange",
                        &(
                            "firewalld-compat".to_string(),
                            protocol.to_string(),
                            start,
                            end,
                            false,
                        ),
                    )
                    .await
                    .map_err(|e| e.to_string())?;
                if error.is_empty() {
                    Ok(rule_id)
                } else {
                    Err(error)
                }
            }
        } else {
            Err(format!("invalid port value: {port}"))
        }
    }

    async fn deregister_rule_id(&self, rule_id: &str) -> Result<bool, String> {
        let proxy = self.services_proxy().await?;
        proxy
            .call("DeregisterServiceRule", &(rule_id.to_string()))
            .await
            .map_err(|e| e.to_string())
    }
}

#[zbus::interface(name = "org.fedoraproject.FirewallD1.zone")]
impl ZoneInterface {
    async fn get_zones(&self) -> zbus::fdo::Result<Vec<String>> {
        let state = self.state.lock().await;
        let mut zones = BTreeSet::new();
        for zone in default_zones() {
            zones.insert(zone);
        }
        for zone in state.zone_interfaces.keys() {
            zones.insert(zone.clone());
        }
        for zone in state.zone_ports.keys() {
            zones.insert(zone.clone());
        }
        Ok(zones.into_iter().collect())
    }

    async fn get_default_zone(&self) -> zbus::fdo::Result<String> {
        Ok(self.state.lock().await.default_zone.clone())
    }

    async fn get_active_zones(&self) -> zbus::fdo::Result<BTreeMap<String, Vec<String>>> {
        let state = self.state.lock().await;
        let mut active = BTreeMap::new();
        for (zone, ifaces) in &state.zone_interfaces {
            if !ifaces.is_empty() {
                active.insert(zone.clone(), ifaces.iter().cloned().collect());
            }
        }
        Ok(active)
    }

    async fn add_port(
        &self,
        #[zbus(signal_emitter)] emitter: zbus::object_server::SignalEmitter<'_>,
        zone: &str,
        port: &str,
        protocol: &str,
        _timeout: u32,
    ) -> zbus::fdo::Result<()> {
        let rule_id = self
            .register_port_rule(protocol, port)
            .await
            .map_err(zbus::fdo::Error::Failed)?;

        let key = RuntimeState::key_for_port(zone, port, protocol);
        let mut state = self.state.lock().await;
        state.port_rule_ids.insert(key, rule_id);
        state
            .zone_ports
            .entry(zone.to_string())
            .or_default()
            .push(PortSpec {
                protocol: protocol.to_string(),
                port: port.to_string(),
            });
        drop(state);

        let _ = Self::port_added(
            &emitter,
            zone.to_string(),
            port.to_string(),
            protocol.to_string(),
        )
        .await;
        let _ = Self::reloaded(&emitter).await;
        Ok(())
    }

    async fn remove_port(
        &self,
        #[zbus(signal_emitter)] emitter: zbus::object_server::SignalEmitter<'_>,
        zone: &str,
        port: &str,
        protocol: &str,
    ) -> zbus::fdo::Result<()> {
        let key = RuntimeState::key_for_port(zone, port, protocol);
        let rule_id = self.state.lock().await.port_rule_ids.remove(&key);
        if let Some(rule_id) = rule_id {
            let _ = self
                .deregister_rule_id(&rule_id)
                .await
                .map_err(zbus::fdo::Error::Failed)?;
        }

        let mut state = self.state.lock().await;
        if let Some(entries) = state.zone_ports.get_mut(zone) {
            entries.retain(|entry| !(entry.protocol == protocol && entry.port == port));
        }
        drop(state);

        let _ = Self::port_removed(
            &emitter,
            zone.to_string(),
            port.to_string(),
            protocol.to_string(),
        )
        .await;
        let _ = Self::reloaded(&emitter).await;
        Ok(())
    }

    async fn add_service(
        &self,
        #[zbus(signal_emitter)] emitter: zbus::object_server::SignalEmitter<'_>,
        zone: &str,
        service: &str,
        _timeout: u32,
    ) -> zbus::fdo::Result<()> {
        let Some(ports) = self.service_db.services.get(service) else {
            return Err(zbus::fdo::Error::Failed(format!(
                "unknown firewalld service '{service}'"
            )));
        };

        let mut rule_ids = Vec::new();
        for port in ports {
            let rule_id = self
                .register_port_rule(&port.protocol, &port.port)
                .await
                .map_err(zbus::fdo::Error::Failed)?;
            rule_ids.push(rule_id);
        }

        let key = RuntimeState::key_for_service(zone, service);
        let mut state = self.state.lock().await;
        state.service_rule_ids.insert(key, rule_ids);
        state
            .zone_services
            .entry(zone.to_string())
            .or_default()
            .insert(service.to_string());
        drop(state);

        let _ = Self::service_added(&emitter, zone.to_string(), service.to_string()).await;
        let _ = Self::reloaded(&emitter).await;
        Ok(())
    }

    async fn remove_service(
        &self,
        #[zbus(signal_emitter)] emitter: zbus::object_server::SignalEmitter<'_>,
        zone: &str,
        service: &str,
    ) -> zbus::fdo::Result<()> {
        let key = RuntimeState::key_for_service(zone, service);
        let rule_ids = self.state.lock().await.service_rule_ids.remove(&key);
        if let Some(rule_ids) = rule_ids {
            for rule_id in rule_ids {
                let _ = self.deregister_rule_id(&rule_id).await;
            }
        }

        let mut state = self.state.lock().await;
        if let Some(services) = state.zone_services.get_mut(zone) {
            services.remove(service);
        }
        drop(state);

        let _ = Self::service_removed(&emitter, zone.to_string(), service.to_string()).await;
        let _ = Self::reloaded(&emitter).await;
        Ok(())
    }

    async fn add_rich_rule(
        &self,
        #[zbus(signal_emitter)] emitter: zbus::object_server::SignalEmitter<'_>,
        zone: &str,
        rule: &str,
        _timeout: u32,
    ) -> zbus::fdo::Result<()> {
        let expr = parse_rich_rule_to_expr(rule).map_err(zbus::fdo::Error::Failed)?;
        let payload = serde_json::json!({
            "family": "inet",
            "table": "palisade",
            "chain": "service-rules",
            "expr": expr,
            "comment": format!("firewalld-rich zone={zone}")
        });

        let proxy = self.services_proxy().await.map_err(zbus::fdo::Error::Failed)?;
        let (rule_id, error): (String, String) = proxy
            .call(
                "RegisterServiceRule",
                &("firewalld-compat".to_string(), payload.to_string(), false),
            )
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        if !error.is_empty() {
            return Err(zbus::fdo::Error::Failed(error));
        }

        let key = RuntimeState::key_for_rich_rule(zone, rule);
        self.state.lock().await.rich_rule_ids.insert(key, rule_id);

        let _ = Self::reloaded(&emitter).await;
        Ok(())
    }

    async fn remove_rich_rule(
        &self,
        #[zbus(signal_emitter)] emitter: zbus::object_server::SignalEmitter<'_>,
        zone: &str,
        rule: &str,
    ) -> zbus::fdo::Result<()> {
        let key = RuntimeState::key_for_rich_rule(zone, rule);
        let maybe_rule_id = self.state.lock().await.rich_rule_ids.remove(&key);
        if let Some(rule_id) = maybe_rule_id {
            let _ = self.deregister_rule_id(&rule_id).await;
        }
        let _ = Self::reloaded(&emitter).await;
        Ok(())
    }

    async fn get_services(&self, zone: &str) -> zbus::fdo::Result<Vec<String>> {
        let state = self.state.lock().await;
        Ok(state
            .zone_services
            .get(zone)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect())
    }

    async fn get_ports(&self, zone: &str) -> zbus::fdo::Result<Vec<String>> {
        let state = self.state.lock().await;
        let ports = state.zone_ports.get(zone).cloned().unwrap_or_default();
        Ok(ports
            .into_iter()
            .map(|port| format!("{}/{}", port.port, port.protocol))
            .collect())
    }

    async fn add_interface(&self, zone: &str, interface: &str) -> zbus::fdo::Result<()> {
        self.state
            .lock()
            .await
            .zone_interfaces
            .entry(zone.to_string())
            .or_default()
            .insert(interface.to_string());
        Ok(())
    }

    async fn remove_interface(&self, zone: &str, interface: &str) -> zbus::fdo::Result<()> {
        if let Some(bindings) = self.state.lock().await.zone_interfaces.get_mut(zone) {
            bindings.remove(interface);
        }
        Ok(())
    }

    async fn change_zone_of_interface(&self, zone: &str, interface: &str) -> zbus::fdo::Result<()> {
        let mut state = self.state.lock().await;
        for ifaces in state.zone_interfaces.values_mut() {
            ifaces.remove(interface);
        }
        state
            .zone_interfaces
            .entry(zone.to_string())
            .or_default()
            .insert(interface.to_string());
        Ok(())
    }

    #[zbus(signal)]
    async fn reloaded(emitter: &zbus::object_server::SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn service_added(
        emitter: &zbus::object_server::SignalEmitter<'_>,
        zone: String,
        service: String,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn service_removed(
        emitter: &zbus::object_server::SignalEmitter<'_>,
        zone: String,
        service: String,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn port_added(
        emitter: &zbus::object_server::SignalEmitter<'_>,
        zone: String,
        port: String,
        protocol: String,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn port_removed(
        emitter: &zbus::object_server::SignalEmitter<'_>,
        zone: String,
        port: String,
        protocol: String,
    ) -> zbus::Result<()>;
}

#[zbus::interface(name = "org.fedoraproject.FirewallD1")]
impl RootInterface {
    async fn reload(&self) -> zbus::fdo::Result<()> {
        Ok(())
    }

    async fn runtime_to_permanent(&self) -> zbus::fdo::Result<()> {
        Ok(())
    }

    async fn complete_reload(&self) -> zbus::fdo::Result<()> {
        Ok(())
    }

    async fn get_log_denied(&self) -> zbus::fdo::Result<String> {
        Ok(self.state.lock().await.log_denied.clone())
    }

    async fn set_log_denied(&self, value: &str) -> zbus::fdo::Result<()> {
        self.state.lock().await.log_denied = value.to_string();
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();

    let service_db = match ServiceDatabase::load() {
        Ok(db) => Arc::new(db),
        Err(err) => {
            warn!(error = %err, "failed to load firewalld service XML database");
            Arc::new(ServiceDatabase::default())
        }
    };

    let state: SharedState = Arc::new(Mutex::new(RuntimeState::new()));
    {
        let mut guard = state.lock().await;
        for zone in default_zones() {
            guard.zone_interfaces.entry(zone).or_default();
        }
    }

    let connection = Connection::system().await?;
    let zone_iface = ZoneInterface {
        state: state.clone(),
        service_db,
        connection: connection.clone(),
    };
    let root_iface = RootInterface {
        state: state.clone(),
    };

    let _conn = zbus::connection::Builder::system()?
        .name("org.fedoraproject.FirewallD1")?
        .serve_at("/org/fedoraproject/FirewallD1", root_iface)?
        .serve_at("/org/fedoraproject/FirewallD1", zone_iface)?
        .build()
        .await?;

    info!("org.fedoraproject.FirewallD1 compatibility shim started");
    tokio::signal::ctrl_c().await?;
    Ok(())
}

fn parse_port_range(input: &str) -> Option<(u32, u32)> {
    if let Some((start, end)) = input.split_once('-') {
        let start = start.trim().parse::<u32>().ok()?;
        let end = end.trim().parse::<u32>().ok()?;
        return Some((start, end));
    }
    let port = input.trim().parse::<u32>().ok()?;
    Some((port, port))
}
