use crate::nftables::changeset::changeset_to_nft_batch;
use crate::nftables::engine::NftEngine;
use crate::nftables::model::{RuleState, RulesetSnapshot};
use crate::nftables::ownership::detect_table_owner;
use crate::safety::audit::AuditLog;
use crate::safety::snapshots::SnapshotManager;
use crate::services::detector::ServiceDetector;
use crate::services::service_registry::{ServiceRuleRecord, ServiceRuleRegistry};
use futures_util::StreamExt;
use palisade_shared::changeset::{ChainSpec, RuleSpec};
use palisade_shared::{Changeset, Operation, Position};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, warn};

#[derive(Clone)]
pub struct ServicesService {
    detector: Arc<ServiceDetector>,
    engine: Arc<NftEngine>,
    snapshots: Arc<SnapshotManager>,
    audit: Arc<Mutex<AuditLog>>,
    registry: Arc<ServiceRuleRegistry>,
}

impl ServicesService {
    pub fn new(
        detector: Arc<ServiceDetector>,
        engine: Arc<NftEngine>,
        snapshots: Arc<SnapshotManager>,
        audit: Arc<Mutex<AuditLog>>,
    ) -> anyhow::Result<Self> {
        let registry = Arc::new(ServiceRuleRegistry::new("/var/lib/palisade/service-rules.db")?);
        Ok(Self {
            detector,
            engine,
            snapshots,
            audit,
            registry,
        })
    }

    pub fn spawn_name_owner_watcher(&self, connection: zbus::Connection) {
        let service = self.clone();
        tokio::spawn(async move {
            let proxy = match zbus::fdo::DBusProxy::new(&connection).await {
                Ok(proxy) => proxy,
                Err(err) => {
                    error!(error = %err, "failed to create DBus proxy for NameOwnerChanged");
                    return;
                }
            };

            let stream = proxy.receive_name_owner_changed().await;
            let Some(mut stream) = stream.ok() else {
                error!("failed to subscribe to NameOwnerChanged");
                return;
            };

            while let Some(signal) = stream.next().await {
                let args = match signal.args() {
                    Ok(args) => args,
                    Err(err) => {
                        warn!(error = %err, "failed to parse NameOwnerChanged args");
                        continue;
                    }
                };

                let new_owner = args
                    .new_owner()
                    .as_ref()
                    .map(|owner| owner.as_str().to_string())
                    .unwrap_or_default();
                if !new_owner.is_empty() {
                    continue;
                }

                let owner = args.name().as_str().to_string();
                if let Err(err) = service
                    .cleanup_temporary_rules_for_owner(&owner, Some(&connection))
                    .await
                {
                    warn!(
                        error = %err,
                        owner = owner,
                        "failed to cleanup temporary service rules"
                    );
                }
            }
        });
    }

    async fn apply_changeset_without_deadman(&self, changeset: &Changeset) -> anyhow::Result<()> {
        let batch = changeset_to_nft_batch(changeset)?;
        self.engine.validate_json_batch(&batch).await?;

        let current = self.engine.list_ruleset().await?;
        let snapshot_id = self.snapshots.create_snapshot(&current)?;

        self.engine.apply_json_batch(&batch).await?;
        self.audit.lock().await.append(
            "service-rule-apply",
            &format!("{}: {}", snapshot_id, changeset.description),
        )?;
        Ok(())
    }

    async fn ensure_service_chain(&self) -> anyhow::Result<()> {
        let raw = self.engine.list_ruleset().await?;
        let snapshot = RulesetSnapshot::from_nft_json(&raw)?;
        let palisade_table = snapshot
            .tables
            .iter()
            .find(|t| t.family == "inet" && t.name == "palisade");

        let mut operations = Vec::new();
        if palisade_table.is_none() {
            operations.push(Operation::AddTable {
                family: "inet".to_string(),
                name: "palisade".to_string(),
            });
        }

        let input_exists = palisade_table
            .and_then(|table| table.chains.iter().find(|c| c.name == "input"))
            .is_some();

        if !input_exists {
            operations.push(Operation::AddChain {
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
        }

        let service_chain_exists = palisade_table
            .and_then(|table| table.chains.iter().find(|c| c.name == "service-rules"))
            .is_some();
        if !service_chain_exists {
            operations.push(Operation::AddChain {
                family: "inet".to_string(),
                table: "palisade".to_string(),
                chain: ChainSpec {
                    name: "service-rules".to_string(),
                    chain_type: None,
                    hook: None,
                    priority: None,
                    policy: None,
                },
            });
        }

        let input_has_jump = palisade_table
            .and_then(|table| table.chains.iter().find(|c| c.name == "input"))
            .map(|input| {
                input
                    .rules
                    .iter()
                    .any(|rule| rule_has_jump_target(rule, "service-rules"))
            })
            .unwrap_or(false);

        if !input_has_jump {
            operations.push(Operation::AddRule {
                family: "inet".to_string(),
                table: "palisade".to_string(),
                chain: "input".to_string(),
                position: Position::Last,
                rule: RuleSpec {
                    expr: vec![serde_json::json!({
                        "jump": {"target": "service-rules"}
                    })],
                    comment: Some("palisade:auto:service-rules-jump".to_string()),
                },
            });
        }

        if operations.is_empty() {
            return Ok(());
        }

        let cs = Changeset {
            version: 1,
            description: "Ensure service registration chains".to_string(),
            operations,
        };
        self.apply_changeset_without_deadman(&cs).await
    }

    async fn resolve_rule_handle_by_comment_tag(&self, comment_tag: &str) -> anyhow::Result<Option<u64>> {
        let raw = self.engine.list_ruleset().await?;
        let snapshot = RulesetSnapshot::from_nft_json(&raw)?;
        let handle = snapshot
            .tables
            .iter()
            .find(|table| table.family == "inet" && table.name == "palisade")
            .and_then(|table| table.chains.iter().find(|chain| chain.name == "service-rules"))
            .and_then(|chain| {
                chain
                    .rules
                    .iter()
                    .filter(|rule| {
                        rule.comment
                            .as_deref()
                            .map(|comment| comment.contains(comment_tag))
                            .unwrap_or(false)
                    })
                    .map(|rule| rule.handle)
                    .max()
            });
        Ok(handle)
    }

    async fn add_service_rule(
        &self,
        service_name: &str,
        protocol: Option<String>,
        port_start: Option<u32>,
        port_end: Option<u32>,
        rule_expr: Vec<serde_json::Value>,
        temporary: bool,
        owner_bus_name: Option<String>,
        raw_rule_json: Option<String>,
    ) -> anyhow::Result<ServiceRuleRecord> {
        if temporary && owner_bus_name.is_none() {
            anyhow::bail!("temporary service rule requires a caller bus name");
        }

        self.ensure_service_chain().await?;
        self.cleanup_stale_temporary_rules().await?;

        let rule_id = format!(
            "srv-{}-{:08x}",
            chrono::Utc::now().timestamp_millis(),
            rand::random::<u32>()
        );
        let comment_tag = format!("palisade:service:{service_name}:{rule_id}");
        let mut comment = comment_tag.clone();
        if let Some(raw) = &raw_rule_json {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) {
                if let Some(existing_comment) = value.get("comment").and_then(|v| v.as_str()) {
                    if !existing_comment.trim().is_empty() {
                        comment = format!("{existing_comment} | {comment_tag}");
                    }
                }
            }
        }

        let cs = Changeset {
            version: 1,
            description: format!("Register service rule {service_name}"),
            operations: vec![Operation::AddRule {
                family: "inet".to_string(),
                table: "palisade".to_string(),
                chain: "service-rules".to_string(),
                position: Position::Last,
                rule: RuleSpec {
                    expr: rule_expr,
                    comment: Some(comment.clone()),
                },
            }],
        };

        self.apply_changeset_without_deadman(&cs).await?;

        let handle = self.resolve_rule_handle_by_comment_tag(&comment_tag).await?;
        let mut record = ServiceRuleRegistry::new_record(
            rule_id,
            service_name.to_string(),
            protocol,
            port_start,
            port_end,
            temporary,
            owner_bus_name,
            comment,
            raw_rule_json,
        );
        record.handle = handle;
        self.registry.upsert(&record)?;
        Ok(record)
    }

    async fn deregister_record(
        &self,
        record: &ServiceRuleRecord,
        connection: Option<&zbus::Connection>,
    ) -> anyhow::Result<bool> {
        let comment_tag = format!("palisade:service:{}:{}", record.service_name, record.rule_id);
        let handle = if let Some(handle) = record.handle {
            Some(handle)
        } else {
            self.resolve_rule_handle_by_comment_tag(&comment_tag).await?
        };

        if let Some(handle) = handle {
            let changeset = Changeset {
                version: 1,
                description: format!("Deregister service rule {}", record.rule_id),
                operations: vec![Operation::DeleteRule {
                    family: "inet".to_string(),
                    table: "palisade".to_string(),
                    chain: "service-rules".to_string(),
                    handle,
                }],
            };
            self.apply_changeset_without_deadman(&changeset).await?;
        }

        let _ = self.registry.delete(&record.rule_id)?;
        if let Some(connection) = connection {
            let _ = emit_service_rule_changed(
                connection,
                &record.service_name,
                "deregistered",
                &record.rule_id,
            )
            .await;
        }
        Ok(true)
    }

    async fn cleanup_temporary_rules_for_owner(
        &self,
        owner_bus_name: &str,
        connection: Option<&zbus::Connection>,
    ) -> anyhow::Result<usize> {
        let temporary = self.registry.list_temporary_for_owner(owner_bus_name)?;
        let mut removed = 0usize;
        for record in temporary {
            if self.deregister_record(&record, connection).await? {
                removed += 1;
            }
        }
        Ok(removed)
    }

    async fn cleanup_stale_temporary_rules(&self) -> anyhow::Result<()> {
        let temporary = self.registry.list_all_temporary()?;
        if temporary.is_empty() {
            return Ok(());
        }

        let connection = zbus::Connection::system().await?;
        let dbus = zbus::fdo::DBusProxy::new(&connection).await?;
        for record in temporary {
            let Some(owner) = record.owner_bus_name.as_deref() else {
                continue;
            };
            let Ok(bus_name) = zbus::names::BusName::try_from(owner) else {
                continue;
            };
            if dbus.get_name_owner(bus_name).await.is_err() {
                let _ = self.deregister_record(&record, Some(&connection)).await;
            }
        }
        Ok(())
    }
}

#[zbus::interface(name = "org.palisade.Daemon1.Services")]
impl ServicesService {
    async fn detect_services(&self) -> zbus::fdo::Result<String> {
        serde_json::to_string(&self.detector.detect())
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
        let value = snapshot
            .tables
            .iter()
            .map(|t| {
                (
                    format!("{}/{}", t.family, t.name),
                    format!("{:?}", detect_table_owner(&t.family, &t.name)),
                )
            })
            .collect::<BTreeMap<_, _>>();
        serde_json::to_string(&value).map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    async fn register_service_port(
        &self,
        #[zbus(signal_emitter)] emitter: zbus::object_server::SignalEmitter<'_>,
        #[zbus(header)] header: zbus::message::Header<'_>,
        service_name: &str,
        protocol: &str,
        port: u32,
        temporary: bool,
    ) -> zbus::fdo::Result<(String, String)> {
        let owner_bus_name = header.sender().map(|sender| sender.to_string());
        let rule_expr = vec![
            serde_json::json!({
                "match": {
                    "left": {"meta": {"key": "l4proto"}},
                    "op": "==",
                    "right": protocol
                }
            }),
            serde_json::json!({
                "match": {
                    "left": {"payload": {"protocol": protocol, "field": "dport"}},
                    "op": "==",
                    "right": port
                }
            }),
            serde_json::json!({ "accept": null }),
        ];

        match self
            .add_service_rule(
                service_name,
                Some(protocol.to_string()),
                Some(port),
                Some(port),
                rule_expr,
                temporary,
                owner_bus_name,
                None,
            )
            .await
        {
            Ok(record) => {
                let _ = Self::service_rule_changed(
                    &emitter,
                    record.service_name.clone(),
                    "registered".to_string(),
                    record.rule_id.clone(),
                )
                .await;
                Ok((record.rule_id, String::new()))
            }
            Err(err) => Ok((String::new(), err.to_string())),
        }
    }

    async fn register_service_port_range(
        &self,
        #[zbus(signal_emitter)] emitter: zbus::object_server::SignalEmitter<'_>,
        #[zbus(header)] header: zbus::message::Header<'_>,
        service_name: &str,
        protocol: &str,
        port_start: u32,
        port_end: u32,
        temporary: bool,
    ) -> zbus::fdo::Result<(String, String)> {
        let owner_bus_name = header.sender().map(|sender| sender.to_string());
        let right = if port_start == port_end {
            serde_json::json!(port_start)
        } else {
            serde_json::json!({ "range": [port_start, port_end] })
        };
        let rule_expr = vec![
            serde_json::json!({
                "match": {
                    "left": {"meta": {"key": "l4proto"}},
                    "op": "==",
                    "right": protocol
                }
            }),
            serde_json::json!({
                "match": {
                    "left": {"payload": {"protocol": protocol, "field": "dport"}},
                    "op": "==",
                    "right": right
                }
            }),
            serde_json::json!({ "accept": null }),
        ];

        match self
            .add_service_rule(
                service_name,
                Some(protocol.to_string()),
                Some(port_start),
                Some(port_end),
                rule_expr,
                temporary,
                owner_bus_name,
                None,
            )
            .await
        {
            Ok(record) => {
                let _ = Self::service_rule_changed(
                    &emitter,
                    record.service_name.clone(),
                    "registered".to_string(),
                    record.rule_id.clone(),
                )
                .await;
                Ok((record.rule_id, String::new()))
            }
            Err(err) => Ok((String::new(), err.to_string())),
        }
    }

    async fn register_service_rule(
        &self,
        #[zbus(signal_emitter)] emitter: zbus::object_server::SignalEmitter<'_>,
        #[zbus(header)] header: zbus::message::Header<'_>,
        service_name: &str,
        rule_json: &str,
        temporary: bool,
    ) -> zbus::fdo::Result<(String, String)> {
        let owner_bus_name = header.sender().map(|sender| sender.to_string());

        let parsed = match parse_advanced_rule_json(rule_json) {
            Ok(parsed) => parsed,
            Err(err) => return Ok((String::new(), err.to_string())),
        };

        match self
            .add_service_rule(
                service_name,
                None,
                None,
                None,
                parsed.0,
                temporary,
                owner_bus_name,
                Some(rule_json.to_string()),
            )
            .await
        {
            Ok(record) => {
                let _ = Self::service_rule_changed(
                    &emitter,
                    record.service_name.clone(),
                    "registered".to_string(),
                    record.rule_id.clone(),
                )
                .await;
                Ok((record.rule_id, String::new()))
            }
            Err(err) => Ok((String::new(), err.to_string())),
        }
    }

    async fn deregister_service_rule(
        &self,
        #[zbus(signal_emitter)] emitter: zbus::object_server::SignalEmitter<'_>,
        rule_id: &str,
    ) -> zbus::fdo::Result<bool> {
        let Some(record) = self
            .registry
            .get(rule_id)
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?
        else {
            return Ok(false);
        };

        self.deregister_record(&record, None)
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        let _ = Self::service_rule_changed(
            &emitter,
            record.service_name,
            "deregistered".to_string(),
            record.rule_id,
        )
        .await;
        Ok(true)
    }

    async fn list_service_rules(&self, service_name: &str) -> zbus::fdo::Result<String> {
        self.cleanup_stale_temporary_rules()
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        let records = self
            .registry
            .list_by_service(service_name)
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;
        serde_json::to_string(&records).map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    async fn list_all_service_rules(&self) -> zbus::fdo::Result<String> {
        self.cleanup_stale_temporary_rules()
            .await
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?;

        let mut grouped = BTreeMap::<String, Vec<ServiceRuleRecord>>::new();
        for record in self
            .registry
            .list_all()
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))?
        {
            grouped
                .entry(record.service_name.clone())
                .or_default()
                .push(record);
        }
        serde_json::to_string(&grouped).map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
    }

    #[zbus(signal)]
    async fn service_rule_changed(
        emitter: &zbus::object_server::SignalEmitter<'_>,
        service_name: String,
        action: String,
        rule_id: String,
    ) -> zbus::Result<()>;
}

fn parse_advanced_rule_json(raw: &str) -> anyhow::Result<(Vec<serde_json::Value>, Option<String>)> {
    let value: serde_json::Value = serde_json::from_str(raw)?;
    let value = if let Some(rule_obj) = value.get("rule") {
        rule_obj.clone()
    } else {
        value
    };

    if let Some(family) = value.get("family").and_then(|v| v.as_str()) {
        if family != "inet" {
            anyhow::bail!("service rules are limited to family=inet");
        }
    }
    if let Some(table) = value.get("table").and_then(|v| v.as_str()) {
        if table != "palisade" {
            anyhow::bail!("service rules are limited to table=palisade");
        }
    }
    if let Some(chain) = value.get("chain").and_then(|v| v.as_str()) {
        if chain != "service-rules" {
            anyhow::bail!("service rules are limited to chain=service-rules");
        }
    }

    if let Some(expr) = value.get("expr").and_then(|v| v.as_array()) {
        return Ok((
            expr.to_vec(),
            value
                .get("comment")
                .and_then(|v| v.as_str())
                .map(ToString::to_string),
        ));
    }

    if let Some(expr) = value.as_array() {
        return Ok((expr.to_vec(), None));
    }

    anyhow::bail!("rule_json must be an expr array or object with expr")
}

fn rule_has_jump_target(rule: &RuleState, target: &str) -> bool {
    rule.expr.iter().any(|expr| {
        expr.get("jump")
            .and_then(|jump| jump.get("target"))
            .and_then(|target_name| target_name.as_str())
            .map(|name| name == target)
            .unwrap_or(false)
    })
}

async fn emit_service_rule_changed(
    connection: &zbus::Connection,
    service_name: &str,
    action: &str,
    rule_id: &str,
) -> zbus::Result<()> {
    connection
        .emit_signal(
            None::<&str>,
            "/org/palisade/Daemon1",
            "org.palisade.Daemon1.Services",
            "ServiceRuleChanged",
            &(service_name.to_string(), action.to_string(), rule_id.to_string()),
        )
        .await
}
