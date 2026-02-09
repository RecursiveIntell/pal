use std::sync::OnceLock;
use tokio::sync::Mutex;
use zbus::{Connection, Proxy};

static SYSTEM_CONN: OnceLock<Mutex<Option<Connection>>> = OnceLock::new();

fn system_conn_lock() -> &'static Mutex<Option<Connection>> {
    SYSTEM_CONN.get_or_init(|| Mutex::new(None))
}

pub async fn get_connection() -> Result<Connection, String> {
    let mut guard = system_conn_lock().lock().await;
    if let Some(conn) = guard.as_ref() {
        return Ok(conn.clone());
    }
    let conn = Connection::system()
        .await
        .map_err(|e| format!("cannot connect to daemon via D-Bus: {e}"))?;
    *guard = Some(conn.clone());
    Ok(conn)
}

async fn get_proxy<'a>(connection: &'a Connection) -> Result<Proxy<'a>, String> {
    Proxy::new(
        connection,
        "org.palisade.Daemon1",
        "/org/palisade/Daemon1",
        "org.palisade.Daemon1",
    )
    .await
    .map_err(|e| format!("daemon proxy error: {e}"))
}

async fn get_services_proxy<'a>(connection: &'a Connection) -> Result<Proxy<'a>, String> {
    Proxy::new(
        connection,
        "org.palisade.Daemon1",
        "/org/palisade/Daemon1",
        "org.palisade.Daemon1.Services",
    )
    .await
    .map_err(|e| format!("daemon services proxy error: {e}"))
}

#[tauri::command]
pub async fn check_daemon_connection() -> Result<bool, String> {
    let conn = get_connection().await?;
    let p = get_proxy(&conn).await?;
    let _: String = p
        .call("ListRuleset", &())
        .await
        .map_err(|e| format!("daemon unreachable: {e}"))?;
    Ok(true)
}

#[tauri::command]
pub async fn list_ruleset() -> Result<String, String> {
    let conn = get_connection().await?;
    let p = get_proxy(&conn).await?;
    p.call("ListRuleset", &()).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_table(family: String, table: String) -> Result<String, String> {
    let conn = get_connection().await?;
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
    let conn = get_connection().await?;
    let p = get_proxy(&conn).await?;
    p.call("GetRuleSummaries", &(family, table, chain))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn validate_changeset(changeset_json: String) -> Result<(bool, String), String> {
    let conn = get_connection().await?;
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
    let conn = get_connection().await?;
    let p = get_proxy(&conn).await?;
    p.call("ApplyChangeset", &(changeset_json, timeout_secs))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn confirm_apply(apply_id: String) -> Result<bool, String> {
    let conn = get_connection().await?;
    let p = get_proxy(&conn).await?;
    p.call("ConfirmApply", &(apply_id))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn rollback_apply(apply_id: String) -> Result<bool, String> {
    let conn = get_connection().await?;
    let p = get_proxy(&conn).await?;
    p.call("RollbackApply", &(apply_id))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_snapshots() -> Result<String, String> {
    let conn = get_connection().await?;
    let p = get_proxy(&conn).await?;
    p.call("ListSnapshots", &())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn create_snapshot() -> Result<(bool, String), String> {
    let conn = get_connection().await?;
    let p = get_proxy(&conn).await?;
    p.call("CreateSnapshot", &())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_snapshot(id: String) -> Result<(bool, String), String> {
    let conn = get_connection().await?;
    let p = get_proxy(&conn).await?;
    p.call("GetSnapshot", &(id))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn restore_snapshot(id: String) -> Result<(bool, String), String> {
    let conn = get_connection().await?;
    let p = get_proxy(&conn).await?;
    p.call("RestoreSnapshot", &(id))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_snapshot(id: String) -> Result<(bool, String), String> {
    let conn = get_connection().await?;
    let p = get_proxy(&conn).await?;
    p.call("DeleteSnapshot", &(id))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_service_rules(service_name: String) -> Result<String, String> {
    let conn = get_connection().await?;
    let p = get_services_proxy(&conn).await?;
    p.call("ListServiceRules", &(service_name))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_all_service_rules() -> Result<String, String> {
    let conn = get_connection().await?;
    let p = get_services_proxy(&conn).await?;
    p.call("ListAllServiceRules", &())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn migrate_firewalld_zones() -> Result<String, String> {
    let conn = get_connection().await?;
    let p = get_proxy(&conn).await?;
    p.call("MigrateFirewalldZones", &())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn switch_firewalld_to_compat() -> Result<(bool, String), String> {
    let conn = get_connection().await?;
    let p = get_proxy(&conn).await?;
    p.call("SwitchFirewalldToCompat", &())
        .await
        .map_err(|e| e.to_string())
}
