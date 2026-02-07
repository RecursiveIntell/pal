use palisade_shared::changeset::{ChainSpec, RuleSpec};
use palisade_shared::{Changeset, Operation, Position};
use roxmltree::Document;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneMigrationSummary {
    pub zone: String,
    pub services: Vec<String>,
    pub ports: Vec<String>,
    pub interfaces: Vec<String>,
    pub source_addresses: Vec<String>,
    pub rules_generated: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationResult {
    pub changeset: Changeset,
    pub zone_summaries: Vec<ZoneMigrationSummary>,
    pub warnings: Vec<String>,
    pub nft_preview: String,
}

#[derive(Debug, Clone, Default)]
struct ZoneConfig {
    name: String,
    services: Vec<String>,
    ports: Vec<PortDecl>,
    interfaces: Vec<String>,
    source_addresses: Vec<String>,
    rich_rules: Vec<RichRule>,
    forward_ports: Vec<ForwardPort>,
    masquerade: bool,
    icmp_blocks: Vec<String>,
}

#[derive(Debug, Clone)]
struct PortDecl {
    protocol: String,
    port: String,
}

#[derive(Debug, Clone, Default)]
struct RichRule {
    family: Option<String>,
    source_address: Option<String>,
    port: Option<PortDecl>,
    action: Option<String>,
}

#[derive(Debug, Clone)]
struct ForwardPort {
    protocol: String,
    from_port: String,
    to_port: Option<String>,
    to_addr: Option<String>,
}

pub fn migrate_firewalld_zones() -> anyhow::Result<MigrationResult> {
    let zone_files = collect_firewalld_zone_files()?;
    let service_db = load_firewalld_service_db()?;
    let mut warnings = Vec::new();
    let mut operations: Vec<Operation> = Vec::new();
    let mut zone_summaries = Vec::new();
    let mut created_chains: BTreeSet<String> = BTreeSet::new();

    if zone_files.is_empty() {
        warnings.push("No firewalld zone XML files were detected.".to_string());
    }

    for zone_path in zone_files {
        let raw = fs::read_to_string(&zone_path)?;
        let zone = match parse_zone_file(&zone_path, &raw) {
            Ok(z) => z,
            Err(err) => {
                warnings.push(format!("{}: {err}", zone_path.display()));
                continue;
            }
        };

        if zone.name.is_empty() {
            warnings.push(format!(
                "{}: skipped unnamed zone file",
                zone_path.display()
            ));
            continue;
        }

        let zone_chain = format!("zone_{}", sanitize_name(&zone.name));
        if created_chains.insert(zone_chain.clone()) {
            operations.push(Operation::AddChain {
                family: "inet".to_string(),
                table: "palisade".to_string(),
                chain: ChainSpec {
                    name: zone_chain.clone(),
                    chain_type: None,
                    hook: None,
                    priority: None,
                    policy: None,
                },
            });
        }

        let mut generated_for_zone = 0usize;

        for iface in &zone.interfaces {
            operations.push(Operation::AddRule {
                family: "inet".to_string(),
                table: "palisade".to_string(),
                chain: "input".to_string(),
                position: Position::Last,
                rule: RuleSpec {
                    expr: vec![
                        serde_json::json!({
                            "match": {
                                "left": {"meta": {"key": "iifname"}},
                                "op": "==",
                                "right": iface
                            }
                        }),
                        serde_json::json!({
                            "jump": {"target": zone_chain}
                        }),
                    ],
                    comment: Some(format!(
                        "firewalld-migrate zone={} interface={}",
                        zone.name, iface
                    )),
                },
            });
            generated_for_zone += 1;
        }

        for service in &zone.services {
            if let Some(ports) = service_db.get(service) {
                for port in ports {
                    operations.push(Operation::AddRule {
                        family: "inet".to_string(),
                        table: "palisade".to_string(),
                        chain: zone_chain.clone(),
                        position: Position::Last,
                        rule: RuleSpec {
                            expr: build_accept_port_expr(
                                &port.protocol,
                                &port.port,
                                zone.source_addresses.first().cloned(),
                            ),
                            comment: Some(format!(
                                "firewalld-migrate zone={} service={}",
                                zone.name, service
                            )),
                        },
                    });
                    generated_for_zone += 1;
                }
            } else {
                warnings.push(format!(
                    "zone={} service={} has no matching service XML entry",
                    zone.name, service
                ));
            }
        }

        for port in &zone.ports {
            operations.push(Operation::AddRule {
                family: "inet".to_string(),
                table: "palisade".to_string(),
                chain: zone_chain.clone(),
                position: Position::Last,
                rule: RuleSpec {
                    expr: build_accept_port_expr(
                        &port.protocol,
                        &port.port,
                        zone.source_addresses.first().cloned(),
                    ),
                    comment: Some(format!(
                        "firewalld-migrate zone={} port={}/{}",
                        zone.name, port.port, port.protocol
                    )),
                },
            });
            generated_for_zone += 1;
        }

        for rich in &zone.rich_rules {
            if let Some(rule) = rich_rule_to_operation(&zone_chain, &zone.name, rich) {
                operations.push(rule);
                generated_for_zone += 1;
            } else {
                warnings.push(format!(
                    "zone={} rich rule could not be translated (family={:?}, source={:?}, action={:?})",
                    zone.name, rich.family, rich.source_address, rich.action
                ));
            }
        }

        if !zone.forward_ports.is_empty() {
            if created_chains.insert("nat_prerouting".to_string()) {
                operations.push(Operation::AddChain {
                    family: "inet".to_string(),
                    table: "palisade".to_string(),
                    chain: ChainSpec {
                        name: "nat_prerouting".to_string(),
                        chain_type: Some("nat".to_string()),
                        hook: Some("prerouting".to_string()),
                        priority: Some(-100),
                        policy: None,
                    },
                });
            }

            for fp in &zone.forward_ports {
                let dnat_expr = build_dnat_expr(fp);
                operations.push(Operation::AddRule {
                    family: "inet".to_string(),
                    table: "palisade".to_string(),
                    chain: "nat_prerouting".to_string(),
                    position: Position::Last,
                    rule: RuleSpec {
                        expr: dnat_expr,
                        comment: Some(format!(
                            "firewalld-migrate zone={} forward-port {}/{}",
                            zone.name, fp.from_port, fp.protocol
                        )),
                    },
                });
                generated_for_zone += 1;
            }
        }

        if zone.masquerade {
            if created_chains.insert("nat_postrouting".to_string()) {
                operations.push(Operation::AddChain {
                    family: "inet".to_string(),
                    table: "palisade".to_string(),
                    chain: ChainSpec {
                        name: "nat_postrouting".to_string(),
                        chain_type: Some("nat".to_string()),
                        hook: Some("postrouting".to_string()),
                        priority: Some(100),
                        policy: None,
                    },
                });
            }

            operations.push(Operation::AddRule {
                family: "inet".to_string(),
                table: "palisade".to_string(),
                chain: "nat_postrouting".to_string(),
                position: Position::Last,
                rule: RuleSpec {
                    expr: vec![serde_json::json!({ "masquerade": null })],
                    comment: Some(format!("firewalld-migrate zone={} masquerade", zone.name)),
                },
            });
            generated_for_zone += 1;
        }

        for icmp in &zone.icmp_blocks {
            warnings.push(format!(
                "zone={} icmp-block={} translated as generic drop rule (type-specific ICMP matching is not yet implemented)",
                zone.name, icmp
            ));
            operations.push(Operation::AddRule {
                family: "inet".to_string(),
                table: "palisade".to_string(),
                chain: zone_chain.clone(),
                position: Position::Last,
                rule: RuleSpec {
                    expr: vec![serde_json::json!({ "drop": null })],
                    comment: Some(format!(
                        "firewalld-migrate zone={} icmp-block={}",
                        zone.name, icmp
                    )),
                },
            });
            generated_for_zone += 1;
        }

        zone_summaries.push(ZoneMigrationSummary {
            zone: zone.name.clone(),
            services: zone.services.clone(),
            ports: zone
                .ports
                .iter()
                .map(|p| format!("{}/{}", p.port, p.protocol))
                .collect(),
            interfaces: zone.interfaces.clone(),
            source_addresses: zone.source_addresses.clone(),
            rules_generated: generated_for_zone,
        });
    }

    let changeset = Changeset {
        version: 1,
        description: "firewalld zone migration".to_string(),
        operations: dedupe_operations(operations),
    };

    let nft_preview = render_nft_preview(&changeset);
    Ok(MigrationResult {
        changeset,
        zone_summaries,
        warnings,
        nft_preview,
    })
}

fn collect_firewalld_zone_files() -> anyhow::Result<Vec<PathBuf>> {
    let mut files = BTreeMap::<String, PathBuf>::new();

    for base in ["/usr/lib/firewalld/zones", "/etc/firewalld/zones"] {
        let dir = Path::new(base);
        if !dir.exists() {
            continue;
        }
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("xml") {
                continue;
            }
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                files.insert(name.to_string(), path);
            }
        }
    }

    Ok(files.into_values().collect())
}

fn load_firewalld_service_db() -> anyhow::Result<BTreeMap<String, Vec<PortDecl>>> {
    let mut entries = BTreeMap::<String, PathBuf>::new();

    for base in ["/usr/lib/firewalld/services", "/etc/firewalld/services"] {
        let dir = Path::new(base);
        if !dir.exists() {
            continue;
        }
        for item in fs::read_dir(dir)? {
            let item = item?;
            let path = item.path();
            if path.extension().and_then(|e| e.to_str()) != Some("xml") {
                continue;
            }
            if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
                entries.insert(file_name.to_string(), path);
            }
        }
    }

    let mut db = BTreeMap::<String, Vec<PortDecl>>::new();
    for (file_name, path) in entries {
        let raw = match fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(_) => continue,
        };
        let Ok(doc) = Document::parse(&raw) else {
            continue;
        };
        let mut ports = Vec::new();
        for node in doc.descendants().filter(|n| n.has_tag_name("port")) {
            let protocol = node.attribute("protocol").unwrap_or("tcp").to_string();
            let port = node.attribute("port").unwrap_or("").to_string();
            if !port.is_empty() {
                ports.push(PortDecl { protocol, port });
            }
        }
        if !ports.is_empty() {
            let service_name = file_name.trim_end_matches(".xml").to_string();
            db.insert(service_name, ports);
        }
    }
    Ok(db)
}

fn parse_zone_file(path: &Path, raw: &str) -> anyhow::Result<ZoneConfig> {
    let doc = Document::parse(raw)?;
    let zone = doc
        .descendants()
        .find(|n| n.has_tag_name("zone"))
        .ok_or_else(|| anyhow::anyhow!("missing <zone> root in {}", path.display()))?;

    let mut config = ZoneConfig::default();
    config.name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("zone")
        .to_string();

    for node in zone.children().filter(|n| n.is_element()) {
        if node.has_tag_name("service") {
            if let Some(name) = node.attribute("name") {
                config.services.push(name.to_string());
            }
            continue;
        }

        if node.has_tag_name("port") {
            let protocol = node.attribute("protocol").unwrap_or("tcp").to_string();
            let port = node.attribute("port").unwrap_or("").to_string();
            if !port.is_empty() {
                config.ports.push(PortDecl { protocol, port });
            }
            continue;
        }

        if node.has_tag_name("interface") {
            if let Some(name) = node.attribute("name") {
                config.interfaces.push(name.to_string());
            }
            continue;
        }

        if node.has_tag_name("source") {
            if let Some(address) = node.attribute("address") {
                config.source_addresses.push(address.to_string());
            }
            continue;
        }

        if node.has_tag_name("forward-port") {
            config.forward_ports.push(ForwardPort {
                protocol: node.attribute("protocol").unwrap_or("tcp").to_string(),
                from_port: node.attribute("port").unwrap_or("").to_string(),
                to_port: node.attribute("to-port").map(ToString::to_string),
                to_addr: node.attribute("to-addr").map(ToString::to_string),
            });
            continue;
        }

        if node.has_tag_name("masquerade") {
            config.masquerade = true;
            continue;
        }

        if node.has_tag_name("icmp-block") {
            if let Some(name) = node.attribute("name") {
                config.icmp_blocks.push(name.to_string());
            }
            continue;
        }

        if node.has_tag_name("rule") {
            let mut rich = RichRule {
                family: node.attribute("family").map(ToString::to_string),
                ..RichRule::default()
            };
            for rule_child in node.children().filter(|c| c.is_element()) {
                if rule_child.has_tag_name("source") {
                    rich.source_address = rule_child.attribute("address").map(ToString::to_string);
                } else if rule_child.has_tag_name("port") {
                    let protocol = rule_child.attribute("protocol").unwrap_or("tcp").to_string();
                    let port = rule_child.attribute("port").unwrap_or("").to_string();
                    if !port.is_empty() {
                        rich.port = Some(PortDecl { protocol, port });
                    }
                } else if rule_child.has_tag_name("accept")
                    || rule_child.has_tag_name("drop")
                    || rule_child.has_tag_name("reject")
                {
                    rich.action = Some(rule_child.tag_name().name().to_string());
                }
            }
            config.rich_rules.push(rich);
        }
    }

    Ok(config)
}

fn rich_rule_to_operation(zone_chain: &str, zone_name: &str, rich: &RichRule) -> Option<Operation> {
    let action = rich.action.clone()?;
    let verdict = match action.as_str() {
        "accept" => serde_json::json!({ "accept": null }),
        "drop" => serde_json::json!({ "drop": null }),
        "reject" => serde_json::json!({ "reject": { "type": "icmpx" } }),
        _ => return None,
    };

    let mut expr = Vec::new();
    if let Some(source) = &rich.source_address {
        expr.push(serde_json::json!({
            "match": {
                "left": {"payload": {"protocol": "ip", "field": "saddr"}},
                "op": "==",
                "right": source
            }
        }));
    }
    if let Some(port) = &rich.port {
        expr.extend(build_protocol_port_matches(&port.protocol, &port.port));
    }
    expr.push(verdict);

    Some(Operation::AddRule {
        family: "inet".to_string(),
        table: "palisade".to_string(),
        chain: zone_chain.to_string(),
        position: Position::Last,
        rule: RuleSpec {
            expr,
            comment: Some(format!(
                "firewalld-migrate zone={} rich-rule action={}",
                zone_name, action
            )),
        },
    })
}

fn build_accept_port_expr(protocol: &str, port: &str, source_addr: Option<String>) -> Vec<serde_json::Value> {
    let mut expr = Vec::new();
    if let Some(source_addr) = source_addr {
        expr.push(serde_json::json!({
            "match": {
                "left": {"payload": {"protocol": "ip", "field": "saddr"}},
                "op": "==",
                "right": source_addr
            }
        }));
    }

    expr.extend(build_protocol_port_matches(protocol, port));
    expr.push(serde_json::json!({ "accept": null }));
    expr
}

fn build_protocol_port_matches(protocol: &str, port: &str) -> Vec<serde_json::Value> {
    let mut expr = vec![serde_json::json!({
        "match": {
            "left": {"meta": {"key": "l4proto"}},
            "op": "==",
            "right": protocol
        }
    })];

    if let Some((start, end)) = parse_port_range(port) {
        if start == end {
            expr.push(serde_json::json!({
                "match": {
                    "left": {"payload": {"protocol": protocol, "field": "dport"}},
                    "op": "==",
                    "right": start
                }
            }));
        } else {
            expr.push(serde_json::json!({
                "match": {
                    "left": {"payload": {"protocol": protocol, "field": "dport"}},
                    "op": "==",
                    "right": {"range": [start, end]}
                }
            }));
        }
    }
    expr
}

fn build_dnat_expr(port: &ForwardPort) -> Vec<serde_json::Value> {
    let mut expr = build_protocol_port_matches(&port.protocol, &port.from_port);

    let to_port = port
        .to_port
        .as_deref()
        .and_then(|p| parse_port_range(p))
        .map(|(start, _)| start);
    let mut dnat_obj = serde_json::Map::new();
    if let Some(to_addr) = &port.to_addr {
        dnat_obj.insert("addr".to_string(), serde_json::Value::String(to_addr.clone()));
    }
    if let Some(to_port) = to_port {
        dnat_obj.insert(
            "port".to_string(),
            serde_json::Value::Number(serde_json::Number::from(to_port)),
        );
    }
    let dnat = serde_json::json!({
        "dnat": serde_json::Value::Object(dnat_obj)
    });
    expr.push(dnat);
    expr
}

fn parse_port_range(port: &str) -> Option<(u32, u32)> {
    if let Some((start, end)) = port.split_once('-') {
        let start = start.trim().parse::<u32>().ok()?;
        let end = end.trim().parse::<u32>().ok()?;
        return Some((start, end));
    }
    let p = port.trim().parse::<u32>().ok()?;
    Some((p, p))
}

fn sanitize_name(raw: &str) -> String {
    raw.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .to_lowercase()
}

fn dedupe_operations(operations: Vec<Operation>) -> Vec<Operation> {
    let mut seen = BTreeSet::new();
    let mut out = Vec::new();
    for op in operations {
        if let Ok(sig) = serde_json::to_string(&op) {
            if seen.insert(sig) {
                out.push(op);
            }
        } else {
            out.push(op);
        }
    }
    out
}

fn render_nft_preview(changeset: &Changeset) -> String {
    let mut lines = Vec::new();
    lines.push("# Generated migration preview".to_string());
    for op in &changeset.operations {
        match op {
            Operation::AddChain { family, table, chain } => {
                lines.push(format!("add chain {} {} {}", family, table, chain.name));
            }
            Operation::AddRule {
                family,
                table,
                chain,
                rule,
                ..
            } => {
                let comment = rule
                    .comment
                    .as_ref()
                    .map(|c| format!(" comment \"{}\"", c))
                    .unwrap_or_default();
                lines.push(format!(
                    "add rule {} {} {} ...{}",
                    family, table, chain, comment
                ));
            }
            _ => {}
        }
    }
    lines.join("\n")
}
