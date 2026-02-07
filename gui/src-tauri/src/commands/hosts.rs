use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Host {
    pub id: String,
    pub name: String,
    pub hostname: String,
    pub port: u16,
    pub username: String,
}

#[tauri::command]
pub async fn list_hosts() -> Result<Vec<Host>, String> {
    Ok(Vec::new())
}
