pub async fn list_remote_ruleset(_host_id: &str) -> anyhow::Result<String> {
    Ok("{\"nftables\":[]}".to_string())
}
