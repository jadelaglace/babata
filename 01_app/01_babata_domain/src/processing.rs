use serde::{Deserialize, Serialize};

use crate::{
    AssetId, DerivativeId, DerivativeKind, ItemId, JobId, LogicalPath, Metadata, ProcessingState,
    RevisionId, RunId, Sha256, UtcTimestamp,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PipelineId(pub String);

impl PipelineId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for PipelineId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// One processing attempt against a C0 revision. Retries create new rows.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessRun {
    pub id: RunId,
    pub pipeline_id: PipelineId,
    pub input_revision_id: RevisionId,
    pub input_item_id: Option<ItemId>,
    pub input_sha256: Sha256,
    pub state: ProcessingState,
    pub provider: String,
    pub tool_or_model: Option<String>,
    pub tool_version: Option<String>,
    pub attempt: u32,
    pub retry_of_run_id: Option<RunId>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub params: Metadata,
    pub usage: Metadata,
    pub loss_notes: Option<String>,
    pub created_at: UtcTimestamp,
    pub started_at: Option<UtcTimestamp>,
    pub finished_at: Option<UtcTimestamp>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobRef {
    pub id: JobId,
    pub state: ProcessingState,
}

/// Machine-produced C1 output. Multiple derivatives may share one run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DerivativeRef {
    pub id: DerivativeId,
    pub run_id: RunId,
    pub kind: DerivativeKind,
    pub output_sha256: Option<Sha256>,
    pub content_text: Option<String>,
    pub content_json: Option<String>,
    pub logical_path: Option<LogicalPath>,
    pub media_type: Option<String>,
    pub language: Option<String>,
    pub input_asset_id: Option<AssetId>,
    pub loss_notes: Option<String>,
    pub metadata: Metadata,
    pub created_at: UtcTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProviderTaskRef {
    pub provider: String,
    pub task_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn derivative_kind_wire_stable() {
        assert_eq!(
            serde_json::to_string(&DerivativeKind::StructuredResult).unwrap(),
            "\"structured_result\""
        );
        assert_eq!(
            serde_json::to_string(&DerivativeKind::MediaMetadata).unwrap(),
            "\"media_metadata\""
        );
    }
}

