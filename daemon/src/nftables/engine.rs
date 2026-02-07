use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct NftEngine {
    nft_bin: String,
}

impl NftEngine {
    pub fn new(nft_bin: impl Into<String>) -> Self {
        Self {
            nft_bin: nft_bin.into(),
        }
    }

    pub async fn list_ruleset(&self) -> anyhow::Result<String> {
        self.run_capture(&["-j", "list", "ruleset"], None).await
    }

    pub async fn list_table(&self, family: &str, table: &str) -> anyhow::Result<String> {
        self.run_capture(&["-j", "list", "table", family, table], None)
            .await
    }

    pub async fn validate_json_batch(&self, batch_json: &str) -> anyhow::Result<()> {
        self.run_capture(&["-c", "-j", "-f", "-"], Some(batch_json))
            .await?;
        Ok(())
    }

    pub async fn apply_json_batch(&self, batch_json: &str) -> anyhow::Result<()> {
        self.run_capture(&["-j", "-f", "-"], Some(batch_json))
            .await?;
        Ok(())
    }

    pub async fn apply_text_file(&self, path: &str) -> anyhow::Result<()> {
        self.run_capture(&["-j", "-f", path], None).await?;
        Ok(())
    }

    async fn run_capture(&self, args: &[&str], stdin: Option<&str>) -> anyhow::Result<String> {
        let mut cmd = Command::new(&self.nft_bin);
        cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
        if stdin.is_some() {
            cmd.stdin(Stdio::piped());
        }

        let mut child = cmd.spawn()?;
        if let Some(input) = stdin {
            if let Some(mut writer) = child.stdin.take() {
                writer.write_all(input.as_bytes()).await?;
            }
        }

        let output = child.wait_with_output().await?;
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            anyhow::bail!("nft command failed: {stderr}")
        }
    }
}
