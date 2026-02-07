use std::process::Command;

pub fn is_active() -> bool {
    Command::new("systemctl")
        .arg("is-active")
        .arg("--quiet")
        .arg("docker")
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}
