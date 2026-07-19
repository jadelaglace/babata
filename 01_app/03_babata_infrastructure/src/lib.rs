pub mod assets;
pub mod backup;
pub mod capabilities;
pub mod config;
pub mod observability;
pub mod paths;
pub mod processing;
pub mod security;
pub mod sources;
pub mod sqlite;
pub mod tools;
pub mod views;

pub use assets::FileAssetStore;
pub use capabilities::StaticCapabilityRegistry;
pub use config::{AppConfig, DataRoot, SqliteOptions, load_config};
pub use observability::SystemClock;
#[cfg(feature = "test-support")]
pub use sqlite::test_support;
pub use sqlite::{
    RawStatus, SqliteDerivedRepository, SqliteRawRepository, open_collection_database,
    open_derived_database, open_raw_database, raw_status,
};

