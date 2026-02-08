use crate::nftables::changeset::changeset_to_nft_batch;
use crate::nftables::engine::NftEngine;
use crate::nftables::model::RulesetSnapshot;
use crate::nftables::ownership::detect_table_owner;
use crate::nftables::summarizer::summarize_expr;
use crate::safety::anti_lockout::{detect_ssh_sessions, evaluate_lockout_risk, LockoutRisk};
use crate::safety::audit::AuditLog;
use crate::safety::dead_man::DeadManSwitch;
use crate::safety::snapshots::SnapshotManager;
use crate::services::detector::ServiceDetector;
use crate::services::firewalld_migrate::migrate_firewalld_zones;
use palisade_shared::changeset::{ChainSpec, RuleSpec};
use palisade_shared::{Changeset, Operation, Position, RuleSummary};
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
            match risk {
                LockoutRisk::Blocking(message) => {
                    return Ok((
                        String::new(),
                        format!("anti-lockout blocked apply: {message}"),
                    ));
                }
                LockoutRisk::Warning(message) => {
                    let _ = self
                        .audit
                        .lock()
                        .await
                        .append("anti_lockout_warning", &message);
                }
            }
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

        if let Err(e) = self.engine.apply_json_batch(&batch).await {
            let _ = self.dead_man.disarm(&apply_id).await;
            self.apply_snapshots.lock().await.remove(&apply_id);
            return Ok((String::new(), format!("apply failed: {e}")));
        }

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
        let mut result =
            migrate_firewalld_zones().map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        let current = self
            .engine
            .list_ruleset()
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        let snapshot = RulesetSnapshot::from_nft_json(&current)
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        let palisade_table = snapshot
            .tables
            .iter()
            .find(|t| t.family == "inet" && t.name == "palisade");
        let has_palisade_table = palisade_table.is_some();
        let has_input_chain = palisade_table
            .map(|t| t.chains.iter().any(|c| c.name == "input"))
            .unwrap_or(false);
        let existing_chain_names = palisade_table
            .map(|t| {
                t.chains
                    .iter()
                    .map(|c| c.name.clone())
                    .collect::<std::collections::HashSet<_>>()
            })
            .unwrap_or_default();

        let mut filtered_ops = Vec::with_capacity(result.changeset.operations.len());
        for op in result.changeset.operations {
            match &op {
                Operation::AddChain {
                    family,
                    table,
                    chain,
                } if family == "inet"
                    && table == "palisade"
                    && existing_chain_names.contains(&chain.name) =>
                {
                    result.warnings.push(format!(
                        "Chain inet/palisade/{} already exists; skipping add-chain operation.",
                        chain.name
                    ));
                }
                _ => filtered_ops.push(op),
            }
        }
        result.changeset.operations = filtered_ops;

        let mut bootstrap = Vec::<Operation>::new();
        if !has_palisade_table {
            bootstrap.push(Operation::AddTable {
                family: "inet".to_string(),
                name: "palisade".to_string(),
            });
            result
                .warnings
                .push("No inet/palisade table found; migration will create it.".to_string());
        }

        if !has_input_chain {
            bootstrap.push(Operation::AddChain {
                family: "inet".to_string(),
                table: "palisade".to_string(),
                chain: ChainSpec {
                    name: "input".to_string(),
                    chain_type: Some("filter".to_string()),
                    hook: Some("input".to_string()),
                    priority: Some(0),
                    policy: Some("drop".to_string()),
                },
            });
            bootstrap.push(Operation::AddRule {
                family: "inet".to_string(),
                table: "palisade".to_string(),
                chain: "input".to_string(),
                position: Position::Last,
                rule: RuleSpec {
                    expr: vec![
                        serde_json::json!({
                            "match": {
                                "left": { "ct": { "key": "state" } },
                                "op": "==",
                                "right": { "set": ["established", "related"] }
                            }
                        }),
                        serde_json::json!({ "accept": null }),
                    ],
                    comment: Some("palisade:bootstrap:allow-established-related".to_string()),
                },
            });
            bootstrap.push(Operation::AddRule {
                family: "inet".to_string(),
                table: "palisade".to_string(),
                chain: "input".to_string(),
                position: Position::Last,
                rule: RuleSpec {
                    expr: vec![
                        serde_json::json!({
                            "match": {
                                "left": { "meta": { "key": "iifname" } },
                                "op": "==",
                                "right": "lo"
                            }
                        }),
                        serde_json::json!({ "accept": null }),
                    ],
                    comment: Some("palisade:bootstrap:allow-loopback".to_string()),
                },
            });
            result.warnings.push(
                "No palisade input base chain found; migration will create it with baseline loopback and established/related accepts."
                    .to_string(),
            );
        }

        if !bootstrap.is_empty() {
            let bootstrap_preview = bootstrap
                .iter()
                .filter_map(|op| match op {
                    Operation::AddTable { family, name } => {
                        Some(format!("add table {} {}", family, name))
                    }
                    Operation::AddChain {
                        family,
                        table,
                        chain,
                    } => Some(format!("add chain {} {} {}", family, table, chain.name)),
                    Operation::AddRule {
                        family,
                        table,
                        chain,
                        rule,
                        ..
                    } => Some(format!(
                        "add rule {} {} {} ...{}",
                        family,
                        table,
                        chain,
                        rule.comment
                            .as_ref()
                            .map(|c| format!(" comment \"{c}\""))
                            .unwrap_or_default()
                    )),
                    _ => None,
                })
                .collect::<Vec<_>>();
            if !bootstrap_preview.is_empty() {
                let prefix = format!(
                    "# Auto-bootstrap for migration\n{}",
                    bootstrap_preview.join("\n")
                );
                result.nft_preview = if result.nft_preview.is_empty() {
                    prefix
                } else {
                    format!("{prefix}\n{}", result.nft_preview)
                };
            }

            let mut operations = bootstrap;
            operations.extend(result.changeset.operations.clone());
            result.changeset.operations = operations;
        }

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
