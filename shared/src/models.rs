use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TableOwner {
    Palisade,
    Docker,
    Tailscale,
    Fail2ban,
    Firewalld,
    Wireguard,
    Unmanaged,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AddressFamily {
    Ip,
    Ip6,
    Inet,
    Arp,
    Bridge,
    Netdev,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum HostStatus {
    Connected { latency_ms: u64 },
    Disconnected { reason: String },
    Drifted { diff: String },
    Error { message: String },
}
