use std::path::PathBuf;

use crate::paths::{DataPaths, ensure_layout};

#[derive(Debug, Clone)]
pub struct DataRoot(pub PathBuf);

#[derive(Debug, Clone)]
pub struct SqliteOptions {
    pub busy_timeout_ms: u64,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub data_root: DataRoot,
    pub sqlite: SqliteOptions,
}

impl AppConfig {
    pub fn paths(&self) -> DataPaths {
        DataPaths::new(self.data_root.0.clone())
    }
}

pub fn load_config() -> Result<AppConfig, std::io::Error> {
    let root = std::env::var_os("BABATA_DATA_HOME").map_or_else(
        || PathBuf::from(r"C:\Users\Aiano\BabataData"),
        PathBuf::from,
    );
    let config = AppConfig {
        data_root: DataRoot(root),
        sqlite: SqliteOptions {
            busy_timeout_ms: 5_000,
        },
    };
    ensure_layout(&config.paths())?;
    Ok(config)
}
