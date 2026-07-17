use serde::{Deserialize, Serialize};

use crate::{ItemId, SublibraryId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SublibraryDefinition {
    pub id: SublibraryId,
    pub title: String,
    pub version: u32,
    pub include: Vec<ItemId>,
    pub exclude: Vec<ItemId>,
    pub organisation_rules: Vec<String>,
}
