use serde::{Deserialize, Serialize};

use babata_domain::{CandidateEnvelope, SublibraryDefinitionInput};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CaptureRequest {
    pub provider: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentifierRequest {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BrowserSessionRequest {
    pub route_id: String,
    pub source_reference: String,
    pub scope_description: String,
    pub installation_id: String,
    pub candidates: Vec<CandidateEnvelope>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SelectCollectionRequest {
    pub session_id: String,
    pub candidate_ids: Vec<String>,
    pub scope_description: String,
    pub confirmed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RetryCollectionRequest {
    pub session_id: String,
    pub candidate_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CancelCollectionRequest {
    pub session_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecollectRequest {
    pub item_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateSublibraryRequest {
    pub definition: SublibraryDefinitionInput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviseSublibraryRequest {
    pub sublibrary_id: String,
    pub expected_version: u32,
    pub definition: SublibraryDefinitionInput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SublibraryVersionRequest {
    pub sublibrary_id: String,
    pub version: u32,
}
