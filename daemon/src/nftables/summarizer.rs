use palisade_shared::{RuleAction, RuleMatch, RuleSummary};

pub fn summarize_expr(expr: &[serde_json::Value]) -> RuleSummary {
    let mut matches = Vec::new();
    let mut action = RuleAction::Unknown {
        raw: "none".to_string(),
    };

    for item in expr {
        parse_match(item, &mut matches);
        if let Some(a) = parse_action(item) {
            action = a;
        }
    }

    let description = build_description(&matches, &action);
    RuleSummary {
        matches,
        action,
        description,
    }
}

fn parse_match(v: &serde_json::Value, out: &mut Vec<RuleMatch>) {
    if let Some(obj) = v.as_object() {
        if let Some(m) = obj.get("match") {
            let left = m.get("left").unwrap_or(&serde_json::Value::Null);
            let right = m.get("right").unwrap_or(&serde_json::Value::Null);
            let op = m.get("op").and_then(|x| x.as_str()).unwrap_or("==");
            let field = parse_field(left);
            let (value, negated) = parse_value(right, op);
            out.push(RuleMatch {
                field,
                operator: op.to_string(),
                value,
                negated,
            });
        }
    }
}

fn parse_field(left: &serde_json::Value) -> String {
    if let Some(payload) = left.get("payload") {
        let protocol = payload
            .get("protocol")
            .and_then(|x| x.as_str())
            .unwrap_or("ip");
        let field = payload
            .get("field")
            .and_then(|x| x.as_str())
            .unwrap_or("field");
        return normalize_field(&format!("{protocol}_{field}"));
    }
    if let Some(meta) = left.get("meta") {
        return normalize_field(meta.get("key").and_then(|x| x.as_str()).unwrap_or("meta"));
    }
    if let Some(ct) = left.get("ct") {
        return normalize_field(&format!(
            "ct_{}",
            ct.get("key").and_then(|x| x.as_str()).unwrap_or("state")
        ));
    }
    "unknown".to_string()
}

fn parse_value(right: &serde_json::Value, op: &str) -> (String, bool) {
    let negated = op == "!=";

    if let Some(set) = right.get("set") {
        if let Some(name) = set.as_str() {
            return (format!("@{name}"), negated);
        }
        if let Some(arr) = set.as_array() {
            let vals = arr.iter().map(stringify).collect::<Vec<_>>().join(", ");
            return (vals, negated);
        }
    }

    if let Some(range) = right.get("range") {
        let start = stringify(range.get(0).unwrap_or(&serde_json::Value::Null));
        let end = stringify(range.get(1).unwrap_or(&serde_json::Value::Null));
        return (format!("{start}-{end}"), negated);
    }

    if let Some(prefix) = right.get("prefix") {
        return (stringify(prefix), negated);
    }

    (stringify(right), negated)
}

fn normalize_field(input: &str) -> String {
    input
        .replace("ip_saddr", "source_ip")
        .replace("ip_daddr", "dest_ip")
        .replace("ip6_saddr", "source_ip6")
        .replace("ip6_daddr", "dest_ip6")
        .replace("tcp_sport", "source_port")
        .replace("udp_sport", "source_port")
        .replace("tcp_dport", "dest_port")
        .replace("udp_dport", "dest_port")
        .replace("iifname", "input_interface")
        .replace("oifname", "output_interface")
        .replace("iif", "input_interface")
        .replace("oif", "output_interface")
        .replace("l4proto", "layer4_proto")
        .replace("nfproto", "network_family")
}

fn stringify(v: &serde_json::Value) -> String {
    if let Some(s) = v.as_str() {
        return s.to_string();
    }
    if let Some(n) = v.as_u64() {
        return n.to_string();
    }
    if let Some(n) = v.as_i64() {
        return n.to_string();
    }
    if let Some(arr) = v.as_array() {
        return arr.iter().map(stringify).collect::<Vec<_>>().join(", ");
    }
    serde_json::to_string(v).unwrap_or_else(|_| "unknown".to_string())
}

fn parse_action(v: &serde_json::Value) -> Option<RuleAction> {
    let obj = v.as_object()?;

    if obj.contains_key("accept") {
        return Some(RuleAction::Accept);
    }
    if obj.contains_key("drop") {
        return Some(RuleAction::Drop);
    }
    if let Some(reject) = obj.get("reject") {
        let reject_type = reject
            .get("type")
            .and_then(|x| x.as_str())
            .or_else(|| reject.as_str())
            .map(ToString::to_string);
        return Some(RuleAction::Reject { reject_type });
    }
    if let Some(jump) = obj.get("jump") {
        let target = jump
            .get("target")
            .and_then(|x| x.as_str())
            .or_else(|| jump.as_str())
            .unwrap_or("chain")
            .to_string();
        return Some(RuleAction::Jump { target });
    }
    if let Some(goto) = obj.get("goto") {
        let target = goto
            .get("target")
            .and_then(|x| x.as_str())
            .or_else(|| goto.as_str())
            .unwrap_or("chain")
            .to_string();
        return Some(RuleAction::Goto { target });
    }
    if obj.contains_key("return") {
        return Some(RuleAction::Return);
    }
    if obj.contains_key("masquerade") {
        return Some(RuleAction::Masquerade);
    }
    if let Some(snat) = obj.get("snat") {
        return Some(RuleAction::Snat {
            nat_address: snat.get("addr").map(stringify),
            nat_port: snat.get("port").map(stringify),
        });
    }
    if let Some(dnat) = obj.get("dnat") {
        return Some(RuleAction::Dnat {
            nat_address: dnat.get("addr").map(stringify),
            nat_port: dnat.get("port").map(stringify),
        });
    }
    if let Some(redirect) = obj.get("redirect") {
        return Some(RuleAction::Redirect {
            nat_port: redirect.get("port").map(stringify),
        });
    }
    if let Some(log) = obj.get("log") {
        return Some(RuleAction::Log {
            prefix: log
                .get("prefix")
                .and_then(|x| x.as_str())
                .map(ToString::to_string),
        });
    }
    if obj.contains_key("counter") {
        return Some(RuleAction::Counter);
    }
    if let Some(limit) = obj.get("limit") {
        return Some(RuleAction::Limit {
            rate: limit.get("rate").map(stringify),
        });
    }

    None
}

fn build_description(matches: &[RuleMatch], action: &RuleAction) -> String {
    let action_txt = match action {
        RuleAction::Accept => "Accept",
        RuleAction::Drop => "Drop",
        RuleAction::Reject { .. } => "Reject",
        RuleAction::Jump { .. } => "Jump",
        RuleAction::Goto { .. } => "Goto",
        RuleAction::Return => "Return",
        RuleAction::Masquerade => "Masquerade",
        RuleAction::Snat { .. } => "SNAT",
        RuleAction::Dnat { .. } => "DNAT",
        RuleAction::Redirect { .. } => "Redirect",
        RuleAction::Log { .. } => "Log",
        RuleAction::Counter => "Count",
        RuleAction::Limit { .. } => "Limit",
        RuleAction::Unknown { .. } => "Apply",
    };

    let mut subject = "all".to_string();
    let mut ports = Vec::new();
    let mut src = None::<String>;
    let mut iface = None::<String>;
    let mut other = Vec::new();

    for m in matches {
        match m.field.as_str() {
            "dest_port" => ports.push(m.value.clone()),
            "source_ip" | "source_ip6" => src = Some(m.value.clone()),
            "input_interface" => iface = Some(m.value.clone()),
            _ => other.push(render_match(m)),
        }
    }

    if !ports.is_empty() {
        subject = format!("to ports {}", ports.join(", "));
    }
    if let Some(s) = src {
        subject = format!("{subject} from {s}");
    }
    if let Some(i) = iface {
        subject = format!("{subject} on {i}");
    }
    if !other.is_empty() {
        subject = format!("{subject} where {}", other.join(" and "));
    }

    format!("{action_txt} {subject}")
}

fn render_match(m: &RuleMatch) -> String {
    if m.negated {
        format!("{} not {}", m.field, m.value)
    } else if m.operator == "in" {
        format!("{} in {}", m.field, m.value)
    } else {
        format!("{} {} {}", m.field, m.operator, m.value)
    }
}

#[cfg(test)]
mod tests {
    use super::summarize_expr;
    use serde_json::json;

    #[test]
    fn summarizes_basic_rule() {
        let expr = vec![
            json!({"match": {"left": {"payload": {"protocol": "tcp", "field": "dport"}}, "op": "==", "right": 22}}),
            json!({"accept": null}),
        ];
        let summary = summarize_expr(&expr);
        assert!(summary.description.contains("Accept"));
        assert_eq!(summary.matches.len(), 1);
    }

    #[test]
    fn summarizes_set_and_reject() {
        let expr = vec![
            json!({
                "match": {
                    "left": { "payload": { "protocol": "ip", "field": "saddr" } },
                    "op": "in",
                    "right": { "set": "blocked_ips" }
                }
            }),
            json!({"reject": {"type": "icmpx"}}),
        ];
        let summary = summarize_expr(&expr);
        assert!(summary.description.contains("Reject"));
        assert!(summary.description.contains("@blocked_ips"));
    }

    #[test]
    fn summarizes_port_range() {
        let expr = vec![
            json!({
                "match": {
                    "left": { "payload": { "protocol": "tcp", "field": "dport" } },
                    "op": "==",
                    "right": { "range": [1024, 65535] }
                }
            }),
            json!({"drop": null}),
        ];
        let summary = summarize_expr(&expr);
        assert!(summary.description.contains("1024-65535"));
        assert!(summary.description.contains("Drop"));
    }
}
