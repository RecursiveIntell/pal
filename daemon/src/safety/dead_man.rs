use rand::Rng;
use std::collections::HashMap;
use std::process::Command;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct DeadManSwitch {
    default_timeout_secs: u64,
    units: Arc<Mutex<HashMap<String, String>>>,
}

impl DeadManSwitch {
    pub fn new(default_timeout_secs: u64) -> Self {
        Self {
            default_timeout_secs,
            units: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn arm(
        &self,
        snapshot_path: &str,
        timeout_secs: Option<u64>,
    ) -> anyhow::Result<String> {
        let timeout = timeout_secs.unwrap_or(self.default_timeout_secs);
        let apply_id = format!("apply-{}", rand::rng().random::<u32>());
        let unit = format!("palisade-rollback-{apply_id}");
        let status = Command::new("systemd-run")
            .arg("--unit")
            .arg(&unit)
            .arg(format!("--on-active={}s", timeout))
            .arg("nft")
            .arg("-j")
            .arg("-f")
            .arg(snapshot_path)
            .status()?;

        if !status.success() {
            anyhow::bail!("failed to arm dead man switch")
        }

        self.units.lock().await.insert(apply_id.clone(), unit);
        Ok(apply_id)
    }

    pub async fn disarm(&self, apply_id: &str) -> anyhow::Result<bool> {
        let unit = self.units.lock().await.remove(apply_id);
        if let Some(unit) = unit {
            let status = Command::new("systemctl").arg("stop").arg(unit).status()?;
            return Ok(status.success());
        }
        Ok(false)
    }
}
