use palisade_shared::TableOwner;

pub fn detect_table_owner(family: &str, table: &str) -> TableOwner {
    if family == "inet" && table == "palisade" {
        return TableOwner::Palisade;
    }
    if table == "docker-bridges" || table.contains("docker") {
        return TableOwner::Docker;
    }
    if table == "fail2ban" || table.starts_with("addr-set-") {
        return TableOwner::Fail2ban;
    }
    if table == "firewalld" {
        return TableOwner::Firewalld;
    }
    if table.starts_with("ts-") || table.contains("tailscale") {
        return TableOwner::Tailscale;
    }
    if table.contains("wg") || table.contains("wireguard") {
        return TableOwner::Wireguard;
    }
    TableOwner::Unmanaged
}
