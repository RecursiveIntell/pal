#[derive(Debug, Clone)]
pub struct SshClient;

impl SshClient {
    pub async fn connect(_host: &str, _port: u16, _user: &str) -> anyhow::Result<Self> {
        Ok(Self)
    }
}
