use crate::monitor::conntrack::ConntrackEvent;
use palisade_shared::TrafficSummary;

#[derive(Default)]
pub struct FlowAggregator;

impl FlowAggregator {
    pub fn update(&mut self, _event: ConntrackEvent) {}

    pub fn snapshot(&self) -> TrafficSummary {
        TrafficSummary {
            total_bytes_per_sec: 0,
            total_packets_per_sec: 0,
            top_flows: Vec::new(),
            counter_deltas: Vec::new(),
        }
    }
}
