pub mod asset_store;
pub mod backup_driver;
pub mod candidate_runner;
pub mod capability_registry;
pub mod clock;
pub mod collection_repository;
pub mod derived_repository;
pub mod job_repository;
pub mod output_builder;
pub mod process_provider;
pub mod raw_repository;
pub mod read_projection;
pub mod source_adapter;
pub mod view_builder;

pub use asset_store::{AssetStorePort, FinalizeAssetOutcome, StagedAsset};
pub use backup_driver::BackupDriverPort;
pub use candidate_runner::CandidateRunnerPort;
pub use capability_registry::CapabilityRegistryPort;
pub use clock::ClockPort;
pub use collection_repository::CollectionRepositoryPort;
pub use derived_repository::{DerivedRepositoryPort, ProcessCommit};
pub use job_repository::JobRepositoryPort;
pub use output_builder::OutputBuilderPort;
pub use process_provider::{
    ProcessProviderPort, ProviderExecutionOutcome, ProviderExecutionRequest, ProviderIdentity,
};
pub use raw_repository::{
    NewAsset, NewCaptureOperation, NewCollection, NewItem, NewRelation, NewRevision,
    NewRouteEvidence, NewSource, PersistGraph, RawRepositoryPort,
};
pub use read_projection::ReadProjectionPort;
pub use source_adapter::SourceAdapterPort;
pub use view_builder::ViewBuilderPort;
