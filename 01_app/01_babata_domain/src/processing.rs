use serde::{Deserialize, Serialize};

use crate::{DerivativeId, DerivativeKind, JobId, ProcessingState, RevisionId, RunId, Sha256};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PipelineId(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessRun {
    pub id: RunId,
    pub pipeline_id: PipelineId,
    pub input_revision_id: RevisionId,
    pub input_sha256: Sha256,
    pub state: ProcessingState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRef {
    pub id: JobId,
    pub state: ProcessingState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DerivativeRef {
    pub id: DerivativeId,
    pub run_id: RunId,
    pub kind: DerivativeKind,
    pub output_sha256: Option<Sha256>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderTaskRef {
    pub provider: String,
    pub task_id: String,
}
