use crate::nftables::engine::NftEngine;
use palisade_shared::{CounterDelta, FlowEntry, TrafficSummary};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Default)]
struct CounterSnapshot {
    packets: u64,
    bytes: u64,
}

#[derive(Default)]
pub struct CounterPoller {
    previous: HashMap<String, CounterSnapshot>,
}

impl CounterPoller {
    pub async fn poll(&mut self, engine: Arc<NftEngine>) -> anyhow::Result<TrafficSummary> {
        let raw = engine.list_ruleset().await?;
        let parsed: Value = serde_json::from_str(&raw)?;
        let entries = parsed
            .get("nftables")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        let mut current: HashMap<String, CounterSnapshot> = HashMap::new();
        let mut flow_entries = Vec::new();
        let mut counter_deltas = Vec::new();
        let mut total_bytes_per_sec = 0_u64;
        let mut total_packets_per_sec = 0_u64;

        for entry in entries {
            let Some(rule) = entry.get("rule") else {
                continue;
            };

            let Some((packets, bytes)) = extract_counter(rule) else {
                continue;
            };

            let family = rule
                .get("family")
                .and_then(Value::as_str)
                .unwrap_or("inet")
                .to_string();
            let table = rule
                .get("table")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            let chain = rule
                .get("chain")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            let handle = rule
                .get("handle")
                .and_then(Value::as_u64)
                .unwrap_or_default();

            let key = format!("{family}/{table}/{chain}/{handle}");
            let prev = self.previous.get(&key).cloned().unwrap_or_default();
            let packets_per_sec = packets.saturating_sub(prev.packets);
            let bytes_per_sec = bytes.saturating_sub(prev.bytes);

            current.insert(key, CounterSnapshot { packets, bytes });
            if packets_per_sec == 0 && bytes_per_sec == 0 {
                continue;
            }

            total_packets_per_sec = total_packets_per_sec.saturating_add(packets_per_sec);
            total_bytes_per_sec = total_bytes_per_sec.saturating_add(bytes_per_sec);

            let comment = rule
                .get("comment")
                .and_then(Value::as_str)
                .unwrap_or("rule")
                .to_string();

            flow_entries.push(FlowEntry {
                src: format!("{family}/{table}/{chain}"),
                dst: format!("handle:{handle} {comment}"),
                protocol: "rule".to_string(),
                packets_per_sec,
                bytes_per_sec,
            });

            counter_deltas.push(CounterDelta {
                family,
                table,
                chain,
                handle,
                packets_per_sec,
                bytes_per_sec,
            });
        }

        self.previous = current;

        flow_entries.sort_by(|a, b| b.bytes_per_sec.cmp(&a.bytes_per_sec));
        flow_entries.truncate(20);

        counter_deltas.sort_by(|a, b| b.bytes_per_sec.cmp(&a.bytes_per_sec));
        counter_deltas.truncate(50);

        Ok(TrafficSummary {
            total_bytes_per_sec,
            total_packets_per_sec,
            top_flows: flow_entries,
            counter_deltas,
        })
    }
}

fn extract_counter(rule: &Value) -> Option<(u64, u64)> {
    if let Some(counter) = rule.get("counter") {
        let packets = counter.get("packets").and_then(Value::as_u64)?;
        let bytes = counter.get("bytes").and_then(Value::as_u64)?;
        return Some((packets, bytes));
    }

    let expr = rule.get("expr")?.as_array()?;
    for part in expr {
        let Some(counter) = part.get("counter") else {
            continue;
        };
        let packets = counter.get("packets").and_then(Value::as_u64)?;
        let bytes = counter.get("bytes").and_then(Value::as_u64)?;
        return Some((packets, bytes));
    }
    None
}
