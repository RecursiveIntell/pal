use std::process::Command;

pub fn is_active() -> bool {
    Command::new("ip")
        .arg("link")
        .arg("show")
        .arg("type")
        .arg("wireguard")
        .output()
        .map(|o| o.status.success() && !String::from_utf8_lossy(&o.stdout).trim().is_empty())
        .unwrap_or(false)
}
