use zbus::{Connection, Proxy};

async fn get_proxy<'a>(connection: &'a Connection) -> Result<Proxy<'a>, String> {
    Proxy::new(
        connection,
        "org.palisade.Daemon1",
        "/org/palisade/Daemon1",
        "org.palisade.Daemon1",
    )
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_ruleset() -> Result<String, String> {
    let conn = Connection::system().await.map_err(|e| e.to_string())?;
    let p = get_proxy(&conn).await?;
    p.call("ListRuleset", &()).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_table(family: String, table: String) -> Result<String, String> {
    let conn = Connection::system().await.map_err(|e| e.to_string())?;
    let p = get_proxy(&conn).await?;
    p.call("ListTable", &(family, table))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_rule_summaries(
    family: String,
    table: String,
    chain: String,
) -> Result<String, String> {
    let conn = Connection::system().await.map_err(|e| e.to_string())?;
    let p = get_proxy(&conn).await?;
    p.call("GetRuleSummaries", &(family, table, chain))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn validate_changeset(changeset_json: String) -> Result<(bool, String), String> {
    let conn = Connection::system().await.map_err(|e| e.to_string())?;
    let p = get_proxy(&conn).await?;
    p.call("ValidateChangeset", &(changeset_json))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn apply_changeset(
    changeset_json: String,
    timeout_secs: u32,
) -> Result<(String, String), String> {
    let conn = Connection::system().await.map_err(|e| e.to_string())?;
    let p = get_proxy(&conn).await?;
    p.call("ApplyChangeset", &(changeset_json, timeout_secs))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn confirm_apply(apply_id: String) -> Result<bool, String> {
    let conn = Connection::system().await.map_err(|e| e.to_string())?;
    let p = get_proxy(&conn).await?;
    p.call("ConfirmApply", &(apply_id))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn rollback_apply(apply_id: String) -> Result<bool, String> {
    let conn = Connection::system().await.map_err(|e| e.to_string())?;
    let p = get_proxy(&conn).await?;
    p.call("RollbackApply", &(apply_id))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_snapshots() -> Result<String, String> {
    let conn = Connection::system().await.map_err(|e| e.to_string())?;
    let p = get_proxy(&conn).await?;
    p.call("ListSnapshots", &())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_snapshot() -> Result<(bool, String), String> {
    let conn = Connection::system().await.map_err(|e| e.to_string())?;
    let p = get_proxy(&conn).await?;
    p.call("CreateSnapshot", &())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_snapshot(id: String) -> Result<(bool, String), String> {
    let conn = Connection::system().await.map_err(|e| e.to_string())?;
    let p = get_proxy(&conn).await?;
    p.call("GetSnapshot", &(id))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn restore_snapshot(id: String) -> Result<(bool, String), String> {
    let conn = Connection::system().await.map_err(|e| e.to_string())?;
    let p = get_proxy(&conn).await?;
    p.call("RestoreSnapshot", &(id))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_snapshot(id: String) -> Result<(bool, String), String> {
    let conn = Connection::system().await.map_err(|e| e.to_string())?;
    let p = get_proxy(&conn).await?;
    p.call("DeleteSnapshot", &(id))
        .await
        .map_err(|e| e.to_string())
}
