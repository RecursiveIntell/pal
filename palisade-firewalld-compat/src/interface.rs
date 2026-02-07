use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortSpec {
    pub protocol: String,
    pub port: String,
}

#[derive(Debug, Clone, Default)]
pub struct RuntimeState {
    pub default_zone: String,
    pub zone_interfaces: BTreeMap<String, BTreeSet<String>>,
    pub zone_ports: BTreeMap<String, Vec<PortSpec>>,
    pub zone_services: BTreeMap<String, BTreeSet<String>>,
    pub port_rule_ids: HashMap<String, String>,
    pub service_rule_ids: HashMap<String, Vec<String>>,
    pub rich_rule_ids: HashMap<String, String>,
    pub log_denied: String,
}

impl RuntimeState {
    pub fn new() -> Self {
        Self {
            default_zone: "public".to_string(),
            log_denied: "off".to_string(),
            ..Self::default()
        }
    }

    pub fn key_for_port(zone: &str, port: &str, protocol: &str) -> String {
        format!("{zone}:{port}/{protocol}")
    }

    pub fn key_for_service(zone: &str, service: &str) -> String {
        format!("{zone}:{service}")
    }

    pub fn key_for_rich_rule(zone: &str, rule: &str) -> String {
        format!("{zone}:{rule}")
    }
}

pub type SharedState = Arc<Mutex<RuntimeState>>;
