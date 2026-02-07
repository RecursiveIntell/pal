use crate::interface::PortSpec;
use roxmltree::Document;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct ServiceDatabase {
    pub services: BTreeMap<String, Vec<PortSpec>>,
}

impl ServiceDatabase {
    pub fn load() -> anyhow::Result<Self> {
        let mut files = BTreeMap::<String, PathBuf>::new();
        for base in ["/usr/lib/firewalld/services", "/etc/firewalld/services"] {
            let dir = Path::new(base);
            if !dir.exists() {
                continue;
            }
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("xml") {
                    continue;
                }
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    files.insert(name.to_string(), path);
                }
            }
        }

        let mut services = BTreeMap::<String, Vec<PortSpec>>::new();
        for (file_name, path) in files {
            let raw = match fs::read_to_string(&path) {
                Ok(raw) => raw,
                Err(_) => continue,
            };
            let doc = match Document::parse(&raw) {
                Ok(doc) => doc,
                Err(_) => continue,
            };

            let mut ports = Vec::new();
            for node in doc.descendants().filter(|n| n.has_tag_name("port")) {
                let protocol = node.attribute("protocol").unwrap_or("tcp").to_string();
                let port = node.attribute("port").unwrap_or("").to_string();
                if !port.is_empty() {
                    ports.push(PortSpec { protocol, port });
                }
            }

            if !ports.is_empty() {
                services.insert(file_name.trim_end_matches(".xml").to_string(), ports);
            }
        }

        Ok(Self { services })
    }
}
