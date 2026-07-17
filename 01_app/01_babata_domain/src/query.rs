use serde::{Deserialize, Serialize};

use crate::{ContentType, ItemId, SourceKind};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryFilter {
    pub text: Option<String>,
    pub source_kind: Option<SourceKind>,
    pub content_type: Option<ContentType>,
    pub limit: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PageCursor(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordSummary {
    pub item_id: ItemId,
    pub title: Option<String>,
    pub excerpt: Option<String>,
}
