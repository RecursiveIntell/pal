use chrono::Utc;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct SnapshotManager {
    dir: PathBuf,
    keep: usize,
}

impl SnapshotManager {
    pub fn new(dir: impl AsRef<Path>) -> Self {
        Self {
            dir: dir.as_ref().to_path_buf(),
            keep: 50,
        }
    }

    pub fn create_snapshot(&self, ruleset_json: &str) -> anyhow::Result<String> {
        fs::create_dir_all(&self.dir)?;
        let id = format!("snapshot-{}", Utc::now().format("%Y%m%d%H%M%S"));
        let path = self.dir.join(format!("{id}.json"));
        fs::write(path, ruleset_json)?;
        self.prune()?;
        Ok(id)
    }

    pub fn list(&self) -> anyhow::Result<Vec<String>> {
        if !self.dir.exists() {
            return Ok(Vec::new());
        }
        let mut ids = fs::read_dir(&self.dir)?
            .filter_map(Result::ok)
            .filter_map(|e| e.file_name().to_str().map(ToString::to_string))
            .filter(|f| f.ends_with(".json"))
            .map(|f| f.trim_end_matches(".json").to_string())
            .collect::<Vec<_>>();
        ids.sort();
        ids.reverse();
        Ok(ids)
    }

    pub fn path_for_id(&self, id: &str) -> PathBuf {
        if id.ends_with(".json") {
            return self.dir.join(id);
        }
        self.dir.join(format!("{id}.json"))
    }

    #[allow(dead_code)]
    pub fn load(&self, id: &str) -> anyhow::Result<String> {
        Ok(fs::read_to_string(self.path_for_id(id))?)
    }

    pub fn delete(&self, id: &str) -> anyhow::Result<bool> {
        let path = self.path_for_id(id);
        if !path.exists() {
            return Ok(false);
        }
        fs::remove_file(path)?;
        Ok(true)
    }

    pub fn prune(&self) -> anyhow::Result<()> {
        let mut entries = self.list()?;
        if entries.len() <= self.keep {
            return Ok(());
        }
        for stale in entries.drain(self.keep..) {
            let _ = fs::remove_file(self.path_for_id(&stale));
        }
        Ok(())
    }
}
