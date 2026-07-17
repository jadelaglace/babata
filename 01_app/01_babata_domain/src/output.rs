use serde::{Deserialize, Serialize};

use crate::{ItemId, OutputId, SublibraryId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputKind {
    HumanReadable,
    Structured,
    Web,
    Obsidian,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputScope {
    pub item_ids: Vec<ItemId>,
    pub sublibrary_id: Option<SublibraryId>,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputState {
    Queued,
    Building,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputManifestRef {
    pub relative_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputBuild {
    pub id: OutputId,
    pub kind: OutputKind,
    pub scope: OutputScope,
    pub state: OutputState,
    pub manifest: Option<OutputManifestRef>,
}
