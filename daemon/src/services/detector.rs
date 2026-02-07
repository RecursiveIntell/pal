use crate::services::{docker, fail2ban, firewalld, tailscale, wireguard};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ServiceDetector;

impl ServiceDetector {
    pub fn detect(&self) -> Vec<String> {
        let mut out = Vec::new();
        if docker::is_active() {
            out.push("docker".to_string());
        }
        if tailscale::is_active() {
            out.push("tailscale".to_string());
        }
        if fail2ban::is_active() {
            out.push("fail2ban".to_string());
        }
        if firewalld::is_active() {
            out.push("firewalld".to_string());
        }
        if wireguard::is_active() {
            out.push("wireguard".to_string());
        }
        out
    }
}
