use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrafficSummary {
    pub total_bytes_per_sec: u64,
    pub total_packets_per_sec: u64,
    pub top_flows: Vec<FlowEntry>,
    pub counter_deltas: Vec<CounterDelta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowEntry {
    pub src: String,
    pub dst: String,
    pub protocol: String,
    pub bytes_per_sec: u64,
    pub packets_per_sec: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounterDelta {
    pub family: String,
    pub table: String,
    pub chain: String,
    pub handle: u64,
    pub packets_per_sec: u64,
    pub bytes_per_sec: u64,
}
