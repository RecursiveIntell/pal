use std::process::Command;

pub fn is_active() -> bool {
    Command::new("systemctl")
        .arg("is-active")
        .arg("--quiet")
        .arg("fail2ban")
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
