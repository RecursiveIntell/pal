#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub snapshot_dir: String,
    pub audit_log_path: String,
    pub default_timeout_secs: u64,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            snapshot_dir: "/var/lib/palisade/snapshots".to_string(),
            audit_log_path: "/var/lib/palisade/audit.log".to_string(),
            default_timeout_secs: 60,
        }
    }
}
