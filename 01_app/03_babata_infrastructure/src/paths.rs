use std::path::{Path, PathBuf};

use babata_domain::LogicalPath;

#[derive(Debug, Clone)]
pub struct DataPaths {
    root: PathBuf,
}

impl DataPaths {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn raw_database(&self) -> PathBuf {
        self.root.join("01_raw/index/raw.sqlite")
    }
    pub fn derived_database(&self) -> PathBuf {
        self.root.join("02_derived/index/derived.sqlite")
    }
    pub fn derived_index(&self) -> PathBuf {
        self.root.join("02_derived/index")
    }
    pub fn raw_assets(&self) -> PathBuf {
        self.root.join("01_raw/assets/sha256")
    }
    pub fn staging(&self, operation_id: &str) -> PathBuf {
        self.root.join("04_runtime/staging").join(operation_id)
    }
    pub fn journal(&self) -> PathBuf {
        self.root.join("04_runtime/asset-journal")
    }
    pub fn orphan(&self) -> PathBuf {
        self.root.join("01_raw/quarantine/orphans")
    }
    pub fn resolve_logical(&self, path: &LogicalPath) -> Result<PathBuf, std::io::Error> {
        let full = self.root.join(path.as_str());
        if !full.starts_with(&self.root) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "logical path escaped data root",
            ));
        }
        Ok(full)
    }
}

pub fn ensure_layout(paths: &DataPaths) -> Result<(), std::io::Error> {
    for path in [
        paths.root().join("00_inbox"),
        paths.root().join("01_raw/index"),
        paths.raw_assets(),
        paths.derived_index(),
        paths.staging(".keep"),
        paths.journal(),
        paths.orphan(),
        paths.root().join("02_derived"),
        paths.root().join("03_views"),
        paths.root().join("05_logs"),
    ] {
        std::fs::create_dir_all(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn layout_and_logical_paths_stay_inside_temp_root() {
        let temporary = tempdir().unwrap();
        let paths = DataPaths::new(temporary.path().to_path_buf());
        ensure_layout(&paths).unwrap();
        assert!(paths.raw_assets().exists());
        assert!(paths.derived_index().exists());
        assert!(
            paths
                .resolve_logical(&LogicalPath::parse("01_raw/assets/sha256/aa/file").unwrap())
                .unwrap()
                .starts_with(temporary.path())
        );
    }
}
