#![allow(dead_code)]

pub fn default_zones() -> Vec<String> {
    vec!["public".to_string(), "trusted".to_string(), "drop".to_string()]
}

pub fn zone_to_chain(zone: &str) -> String {
    match zone {
        "public" => "input".to_string(),
        "trusted" => "zone_trusted".to_string(),
        "drop" => "zone_drop".to_string(),
        other => format!("zone_{}", sanitize(other)),
    }
}

pub fn chain_to_zone(chain: &str) -> String {
    match chain {
        "input" => "public".to_string(),
        "zone_trusted" => "trusted".to_string(),
        "zone_drop" => "drop".to_string(),
        other if other.starts_with("zone_") => other.trim_start_matches("zone_").to_string(),
        other => other.to_string(),
    }
}

fn sanitize(raw: &str) -> String {
    raw.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
        .to_lowercase()
}
