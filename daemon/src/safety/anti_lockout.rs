use std::process::Command;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SshSession {
    pub local: String,
    pub peer: String,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum LockoutRisk {
    Warning(String),
    Blocking(String),
}

pub fn detect_ssh_sessions() -> anyhow::Result<Vec<SshSession>> {
    let output = Command::new("ss").arg("-tnp").output()?;
    if !output.status.success() {
        anyhow::bail!("ss -tnp failed")
    }

    let text = String::from_utf8_lossy(&output.stdout);
    let sessions = text
        .lines()
        .filter(|l| l.contains("ssh"))
        .filter_map(|line| {
            let cols = line.split_whitespace().collect::<Vec<_>>();
            if cols.len() < 5 {
                return None;
            }
            Some(SshSession {
                local: cols[3].to_string(),
                peer: cols[4].to_string(),
            })
        })
        .collect::<Vec<_>>();

    Ok(sessions)
}

pub fn evaluate_lockout_risk(_proposed_ruleset: &str, sessions: &[SshSession]) -> Option<LockoutRisk> {
    if sessions.is_empty() {
        return Some(LockoutRisk::Warning(
            "No active SSH sessions detected; remote recovery may be unavailable".to_string(),
        ));
    }
    None
}
