use serde::{Deserialize, Serialize};

use crate::{CapabilityStatus, ViewId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViewKind {
    Datasette,
    Obsidian,
    Export,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ViewDescriptor {
    pub id: ViewId,
    pub kind: ViewKind,
    pub status: CapabilityStatus,
    pub activation_phase: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BuildTarget {
    pub view_id: ViewId,
    pub relative_output: String,
}
