use serde::{Deserialize, Serialize};

use crate::{OutputId, SearchRecordDetail, Sha256, SublibraryId, UtcTimestamp};

pub const OUTPUT_MANIFEST_SCHEMA_VERSION: &str = "babata.output-manifest/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputKind {
    HumanReadable,
    Structured,
    Web,
    Obsidian,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SublibraryOutputScope {
    pub sublibrary_id: SublibraryId,
    pub definition_version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputScope {
    #[serde(default)]
    pub record_ids: Vec<String>,
    pub sublibrary: Option<SublibraryOutputScope>,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputState {
    Succeeded,
    Verified,
    Deleted,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputManifestRef {
    pub relative_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputInputRecord {
    pub detail: SearchRecordDetail,
    pub input_sha256: Sha256,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct OutputScoreProfileRef {
    pub profile_id: String,
    pub profile_ordinal: u32,
    pub interest_weight: u8,
    pub strategy_weight: u8,
    pub consensus_weight: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputDocument {
    pub id: OutputId,
    pub kind: OutputKind,
    pub scope: OutputScope,
    pub generated_at: UtcTimestamp,
    pub builder_version: String,
    pub template_version: String,
    pub score_profiles: Vec<OutputScoreProfileRef>,
    pub records: Vec<OutputInputRecord>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputBuild {
    pub id: OutputId,
    pub kind: OutputKind,
    pub scope: OutputScope,
    pub state: OutputState,
    pub generation: u32,
    pub builder_version: String,
    pub template_version: String,
    pub artifact_path: String,
    pub output_sha256: Sha256,
    pub manifest: OutputManifestRef,
    pub differences: Vec<String>,
    pub generated_at: UtcTimestamp,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputVerification {
    pub output_id: OutputId,
    pub valid: bool,
    pub expected_sha256: Sha256,
    pub actual_sha256: Option<Sha256>,
    pub detail: String,
}
