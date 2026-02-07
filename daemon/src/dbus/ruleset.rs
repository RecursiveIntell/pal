use crate::nftables::changeset::changeset_to_nft_batch;
use crate::nftables::engine::NftEngine;
use crate::nftables::model::RulesetSnapshot;
use crate::nftables::ownership::detect_table_owner;
use crate::nftables::summarizer::summarize_expr;
use crate::safety::anti_lockout::{detect_ssh_sessions, evaluate_lockout_risk};
use crate::safety::audit::AuditLog;
use crate::safety::dead_man::DeadManSwitch;
use crate::safety::snapshots::SnapshotManager;
use crate::services::detector::ServiceDetector;
use crate::services::firewalld_migrate::migrate_firewalld_zones;
use palisade_shared::{Changeset, RuleSummary};
use std::collections::HashMap;
use std::process::Command;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct RulesetService {
    engine: Arc<NftEngine>,
    snapshots: Arc<SnapshotManager>,
    dead_man: Arc<DeadManSwitch>,
    audit: Arc<Mutex<AuditLog>>,
    services: Arc<ServiceDetector>,
    apply_snapshots: Arc<Mutex<HashMap<String, String>>>,
}

impl RulesetService {
    pub fn new(
        engine: Arc<NftEngine>,
        snapshots: Arc<SnapshotManager>,
        dead_man: Arc<DeadManSwitch>,
        audit: Arc<Mutex<AuditLog>>,
        services: Arc<ServiceDetector>,
    ) -> Self {
        Self {
            engine,
            snapshots,
            dead_man,
            audit,
            services,
            apply_snapshots: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

#[zbus::interface(name = "org.palisade.Daemon1")]
impl RulesetService {
    async fn list_ruleset(&self) -> zbus::fdo::Result<String> {
        self.engine
            .list_ruleset()
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    async fn list_table(&self, family: &str, table: &str) -> zbus::fdo::Result<String> {
        self.engine
            .list_table(family, table)
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    async fn get_rule_summaries(
        &self,
        family: &str,
        table: &str,
        chain: &str,
    ) -> zbus::fdo::Result<String> {
        let raw = self
            .engine
            .list_table(family, table)
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        let mut snapshot = RulesetSnapshot::from_nft_json(&raw)
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        for t in &mut snapshot.tables {
            t.owner = detect_table_owner(&t.family, &t.name);
        }

        let summaries = snapshot
            .tables
            .iter()
            .flat_map(|t| t.chains.iter())
            .filter(|c| c.name == chain)
            .flat_map(|c| c.rules.iter())
            .map(|r| summarize_expr(&r.expr))
            .collect::<Vec<RuleSummary>>();

        serde_json::to_string(&summaries).map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    async fn validate_changeset(&self, changeset_json: &str) -> zbus::fdo::Result<(bool, String)> {
        let parsed: Changeset = match serde_json::from_str(changeset_json) {
            Ok(v) => v,
            Err(e) => return Ok((false, e.to_string())),
        };

        let batch = match changeset_to_nft_batch(&parsed) {
            Ok(b) => b,
            Err(e) => return Ok((false, e.to_string())),
        };

        match self.engine.validate_json_batch(&batch).await {
            Ok(_) => Ok((true, String::new())),
            Err(e) => Ok((false, e.to_string())),
        }
    }

    async fn apply_changeset(
        &self,
        changeset_json: &str,
        timeout_secs: u32,
    ) -> zbus::fdo::Result<(String, String)> {
        let parsed: Changeset = serde_json::from_str(changeset_json)
            .map_err(|e| zbus::fdo::Error::Failed(format!("invalid changeset: {e}")))?;

        let batch =
            changeset_to_nft_batch(&parsed).map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        self.engine
            .validate_json_batch(&batch)
            .await
            .map_err(|e| zbus::fdo::Error::Failed(format!("dry-run failed: {e}")))?;

        let sessions =
            detect_ssh_sessions().map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        if let Some(risk) = evaluate_lockout_risk(&batch, &sessions) {
            return Ok((String::new(), format!("anti-lockout warning: {risk}")));
        }

        let current = self
            .engine
            .list_ruleset()
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        let snapshot_id = self
            .snapshots
            .create_snapshot(&current)
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        let snapshot_path = self
            .snapshots
            .path_for_id(&snapshot_id)
            .to_string_lossy()
            .to_string();

        let apply_id = self
            .dead_man
            .arm(&snapshot_path, Some(timeout_secs as u64))
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        self.apply_snapshots
            .lock()
            .await
            .insert(apply_id.clone(), snapshot_path.clone());

        self.engine
            .apply_json_batch(&batch)
            .await
            .map_err(|e| zbus::fdo::Error::Failed(format!("apply failed: {e}")))?;

        self.audit
            .lock()
            .await
            .append("apply", &format!("{}: {}", apply_id, parsed.description))
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        Ok((apply_id, String::new()))
    }

    async fn confirm_apply(&self, apply_id: &str) -> zbus::fdo::Result<bool> {
        let ok = self
            .dead_man
            .disarm(apply_id)
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        self.apply_snapshots.lock().await.remove(apply_id);
        Ok(ok)
    }

    async fn rollback_apply(&self, apply_id: &str) -> zbus::fdo::Result<bool> {
        let _ = self
            .dead_man
            .disarm(apply_id)
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        let snapshot_path = self.apply_snapshots.lock().await.remove(apply_id);
        if let Some(path) = snapshot_path {
            self.engine
                .apply_text_file(&path)
                .await
                .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
            return Ok(true);
        }
        Ok(false)
    }

    async fn list_snapshots(&self) -> zbus::fdo::Result<String> {
        let list = self
            .snapshots
            .list()
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        serde_json::to_string(&list).map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    async fn create_snapshot(&self) -> zbus::fdo::Result<(bool, String)> {
        let current = match self.engine.list_ruleset().await {
            Ok(raw) => raw,
            Err(e) => return Ok((false, e.to_string())),
        };

        match self.snapshots.create_snapshot(&current) {
            Ok(id) => Ok((true, id)),
            Err(e) => Ok((false, e.to_string())),
        }
    }

    async fn get_snapshot(&self, id: &str) -> zbus::fdo::Result<(bool, String)> {
        match self.snapshots.load(id) {
            Ok(content) => Ok((true, content)),
            Err(e) => Ok((false, e.to_string())),
        }
    }

    async fn delete_snapshot(&self, id: &str) -> zbus::fdo::Result<(bool, String)> {
        match self.snapshots.delete(id) {
            Ok(true) => Ok((true, String::new())),
            Ok(false) => Ok((false, "snapshot not found".to_string())),
            Err(e) => Ok((false, e.to_string())),
        }
    }

    async fn restore_snapshot(&self, id: &str) -> zbus::fdo::Result<(bool, String)> {
        let path = self.snapshots.path_for_id(id).to_string_lossy().to_string();
        match self.engine.apply_text_file(&path).await {
            Ok(_) => Ok((true, String::new())),
            Err(e) => Ok((false, e.to_string())),
        }
    }

    async fn migrate_firewalld_zones(&self) -> zbus::fdo::Result<String> {
        let result =
            migrate_firewalld_zones().map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        serde_json::to_string(&result).map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    async fn switch_firewalld_to_compat(&self) -> zbus::fdo::Result<(bool, String)> {
        let disable = Command::new("systemctl")
            .arg("disable")
            .arg("--now")
            .arg("firewalld")
            .status();
        match disable {
            Ok(status) if status.success() => {}
            Ok(status) => {
                return Ok((
                    false,
                    format!("failed to disable firewalld (exit {})", status),
                ));
            }
            Err(err) => return Ok((false, err.to_string())),
        }

        let enable = Command::new("systemctl")
            .arg("enable")
            .arg("--now")
            .arg("palisade-firewalld-compat")
            .status();
        match enable {
            Ok(status) if status.success() => Ok((true, String::new())),
            Ok(status) => Ok((
                false,
                format!(
                    "firewalld disabled, but failed to enable palisade-firewalld-compat (exit {})",
                    status
                ),
            )),
            Err(err) => Ok((false, err.to_string())),
        }
    }

    async fn detect_services(&self) -> zbus::fdo::Result<String> {
        serde_json::to_string(&self.services.detect())
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    async fn get_table_ownership(&self) -> zbus::fdo::Result<String> {
        let raw = self
            .engine
            .list_ruleset()
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        let snapshot = RulesetSnapshot::from_nft_json(&raw)
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        let ownership = snapshot
            .tables
            .iter()
            .map(|t| {
                (
                    format!("{}/{}", t.family, t.name),
                    format!("{:?}", detect_table_owner(&t.family, &t.name)).to_lowercase(),
                )
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        serde_json::to_string(&ownership).map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    #[zbus(signal)]
    async fn ruleset_changed(
        emitter: &zbus::object_server::SignalEmitter<'_>,
        change_type: String,
        details: String,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn external_modification(
        emitter: &zbus::object_server::SignalEmitter<'_>,
        description: String,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn counter_update(
        emitter: &zbus::object_server::SignalEmitter<'_>,
        json: String,
    ) -> zbus::Result<()>;
}
