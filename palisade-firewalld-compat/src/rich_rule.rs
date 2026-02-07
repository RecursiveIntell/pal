pub fn parse_rich_rule_to_expr(rule: &str) -> Result<Vec<serde_json::Value>, String> {
    let mut protocol: Option<String> = None;
    let mut port: Option<String> = None;
    let mut source: Option<String> = None;
    let mut action: Option<String> = None;

    if let Some(found) = capture_between(rule, "protocol=\"", "\"") {
        protocol = Some(found.to_string());
    }
    if let Some(found) = capture_between(rule, "port=\"", "\"") {
        port = Some(found.to_string());
    }
    if let Some(found) = capture_between(rule, "address=\"", "\"") {
        source = Some(found.to_string());
    }
    if rule.contains(" accept") || rule.ends_with("accept") {
        action = Some("accept".to_string());
    } else if rule.contains(" drop") || rule.ends_with("drop") {
        action = Some("drop".to_string());
    } else if rule.contains(" reject") || rule.ends_with("reject") {
        action = Some("reject".to_string());
    }

    let action = action.ok_or_else(|| "unsupported rich rule action".to_string())?;
    let mut expr = Vec::new();

    if let Some(source) = source {
        expr.push(serde_json::json!({
            "match": {
                "left": {"payload": {"protocol": "ip", "field": "saddr"}},
                "op": "==",
                "right": source
            }
        }));
    }

    if let Some(protocol) = protocol {
        expr.push(serde_json::json!({
            "match": {
                "left": {"meta": {"key": "l4proto"}},
                "op": "==",
                "right": protocol
            }
        }));
        if let Some(port) = port {
            let right = parse_port_right(&port).unwrap_or_else(|| serde_json::json!(port));
            expr.push(serde_json::json!({
                "match": {
                    "left": {"payload": {"protocol": protocol, "field": "dport"}},
                    "op": "==",
                    "right": right
                }
            }));
        }
    }

    match action.as_str() {
        "accept" => expr.push(serde_json::json!({ "accept": null })),
        "drop" => expr.push(serde_json::json!({ "drop": null })),
        "reject" => expr.push(serde_json::json!({ "reject": { "type": "icmpx" } })),
        _ => return Err("unsupported rich rule action".to_string()),
    }

    Ok(expr)
}

fn capture_between<'a>(input: &'a str, left: &str, right: &str) -> Option<&'a str> {
    let start = input.find(left)? + left.len();
    let rest = &input[start..];
    let end = rest.find(right)?;
    Some(&rest[..end])
}

fn parse_port_right(port: &str) -> Option<serde_json::Value> {
    if let Some((start, end)) = port.split_once('-') {
        let start = start.parse::<u32>().ok()?;
        let end = end.parse::<u32>().ok()?;
        return Some(serde_json::json!({ "range": [start, end] }));
    }
    let single = port.parse::<u32>().ok()?;
    Some(serde_json::json!(single))
}
