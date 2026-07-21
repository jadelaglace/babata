use serde::{Deserialize, Serialize};

use crate::{
    CollectionSessionId, ContentType, DomainError, ItemId, RevisionId, SourceRouteId, UtcTimestamp,
};

pub const COMMON_SOURCE_METADATA_SCHEMA_V1: &str = "babata.c0.common/v1";
pub const SOURCE_MEDIA_METADATA_SCHEMA_V1: &str = "babata.c0.media/v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceAuthor {
    pub display_name: String,
    #[serde(default)]
    pub native_id: Option<String>,
    #[serde(default)]
    pub locator: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceHierarchyNode {
    #[serde(default)]
    pub kind: Option<String>,
    pub name: String,
    #[serde(default)]
    pub native_id: Option<String>,
    #[serde(default)]
    pub locator: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceLimitation {
    pub code: String,
    pub detail: String,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceAccessState {
    Accessible,
    Restricted,
    Inaccessible,
    Removed,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceMediaEntry {
    pub kind: String,
    #[serde(default)]
    pub media_type: Option<String>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub page_count: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceMediaMetadata {
    #[serde(default = "source_media_schema_v1")]
    pub schema: String,
    #[serde(default)]
    pub entries: Vec<SourceMediaEntry>,
}

impl Default for SourceMediaMetadata {
    fn default() -> Self {
        Self {
            schema: source_media_schema_v1(),
            entries: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommonSourceMetadata {
    #[serde(default = "common_source_schema_v1")]
    pub schema: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub authors: Vec<SourceAuthor>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub source_published_at: Option<UtcTimestamp>,
    #[serde(default)]
    pub source_updated_at: Option<UtcTimestamp>,
    #[serde(default)]
    pub hierarchy: Vec<SourceHierarchyNode>,
    #[serde(default)]
    pub context: Option<String>,
    #[serde(default)]
    pub limitations: Vec<SourceLimitation>,
    #[serde(default)]
    pub access_state: SourceAccessState,
    #[serde(default)]
    pub media: SourceMediaMetadata,
}

impl Default for CommonSourceMetadata {
    fn default() -> Self {
        Self {
            schema: common_source_schema_v1(),
            title: None,
            authors: Vec::new(),
            language: None,
            source_published_at: None,
            source_updated_at: None,
            hierarchy: Vec::new(),
            context: None,
            limitations: Vec::new(),
            access_state: SourceAccessState::Unknown,
            media: SourceMediaMetadata::default(),
        }
    }
}

impl CommonSourceMetadata {
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.schema != COMMON_SOURCE_METADATA_SCHEMA_V1 {
            return Err(DomainError::Invalid {
                field: "common_source_metadata.schema",
                value: self.schema.clone(),
            });
        }
        if self.media.schema != SOURCE_MEDIA_METADATA_SCHEMA_V1 {
            return Err(DomainError::Invalid {
                field: "common_source_metadata.media.schema",
                value: self.media.schema.clone(),
            });
        }
        require_optional_text("common_source_metadata.title", self.title.as_deref())?;
        require_optional_text("common_source_metadata.language", self.language.as_deref())?;
        require_optional_text("common_source_metadata.context", self.context.as_deref())?;
        for author in &self.authors {
            require_text(
                "common_source_metadata.authors.display_name",
                &author.display_name,
            )?;
            require_optional_text(
                "common_source_metadata.authors.native_id",
                author.native_id.as_deref(),
            )?;
            require_optional_text(
                "common_source_metadata.authors.locator",
                author.locator.as_deref(),
            )?;
        }
        for node in &self.hierarchy {
            require_text("common_source_metadata.hierarchy.name", &node.name)?;
            require_optional_text(
                "common_source_metadata.hierarchy.kind",
                node.kind.as_deref(),
            )?;
        }
        for limitation in &self.limitations {
            require_text("common_source_metadata.limitations.code", &limitation.code)?;
            require_text(
                "common_source_metadata.limitations.detail",
                &limitation.detail,
            )?;
        }
        for media in &self.media.entries {
            require_text("common_source_metadata.media.kind", &media.kind)?;
            if media.width == Some(0) || media.height == Some(0) || media.page_count == Some(0) {
                return Err(DomainError::Invalid {
                    field: "common_source_metadata.media.dimension",
                    value: "0".to_owned(),
                });
            }
        }
        Ok(())
    }

    #[must_use]
    pub fn with_legacy_fallback(
        mut self,
        title: Option<&str>,
        source_updated_at: Option<&UtcTimestamp>,
        hierarchy: &[String],
        limitations: &[String],
        context: Option<&str>,
    ) -> Self {
        if self.title.is_none() {
            self.title = title
                .filter(|value| !value.trim().is_empty())
                .map(str::to_owned);
        }
        if self.source_updated_at.is_none() {
            self.source_updated_at = source_updated_at.cloned();
        }
        if self.hierarchy.is_empty() {
            self.hierarchy = hierarchy
                .iter()
                .filter(|name| !name.trim().is_empty())
                .map(|name| SourceHierarchyNode {
                    kind: None,
                    name: name.clone(),
                    native_id: None,
                    locator: None,
                })
                .collect();
        }
        if self.limitations.is_empty() {
            self.limitations = limitations
                .iter()
                .filter(|detail| !detail.trim().is_empty())
                .map(|detail| SourceLimitation {
                    code: "provider_reported".to_owned(),
                    detail: detail.clone(),
                })
                .collect();
        }
        if self.context.is_none() {
            self.context = context
                .filter(|value| !value.trim().is_empty())
                .map(str::to_owned);
        }
        self
    }
}

fn common_source_schema_v1() -> String {
    COMMON_SOURCE_METADATA_SCHEMA_V1.to_owned()
}

fn source_media_schema_v1() -> String {
    SOURCE_MEDIA_METADATA_SCHEMA_V1.to_owned()
}

fn require_text(field: &'static str, value: &str) -> Result<(), DomainError> {
    if value.trim().is_empty() {
        Err(DomainError::Empty { field })
    } else {
        Ok(())
    }
}

fn require_optional_text(field: &'static str, value: Option<&str>) -> Result<(), DomainError> {
    value.map_or(Ok(()), |value| require_text(field, value))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateSummary {
    pub candidate_id: String,
    pub session_id: CollectionSessionId,
    pub route_id: SourceRouteId,
    pub source_native_id: Option<String>,
    pub title: Option<String>,
    pub source_location: Option<String>,
    pub hierarchy: Vec<String>,
    pub content_type: ContentType,
    pub source_updated_at: Option<UtcTimestamp>,
    pub attachment_available: Option<bool>,
    pub limitations: Vec<String>,
    pub selection_capabilities: Vec<String>,
    #[serde(default)]
    pub common_metadata: CommonSourceMetadata,
}

impl CandidateSummary {
    pub fn effective_common_metadata(&self) -> CommonSourceMetadata {
        let context = self
            .hierarchy
            .iter()
            .filter(|part| !part.trim().is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(" / ");
        self.common_metadata.clone().with_legacy_fallback(
            self.title.as_deref(),
            self.source_updated_at.as_ref(),
            &self.hierarchy,
            &self.limitations,
            Some(&context),
        )
    }

    #[must_use]
    pub fn with_common_from_legacy(mut self) -> Self {
        self.common_metadata = self.effective_common_metadata();
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectionSelection {
    pub session_id: CollectionSessionId,
    pub candidate_ids: Vec<String>,
    pub scope_description: String,
    pub confirmed: bool,
    pub authorised_context: String,
    pub requested_attachments: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollectionSessionState {
    Discovering,
    AwaitingSelection,
    Running,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectionSession {
    pub session_id: CollectionSessionId,
    pub route_id: SourceRouteId,
    pub source_reference: String,
    pub scope_description: String,
    pub authorisation_id: String,
    pub state: CollectionSessionState,
    pub created_at: UtcTimestamp,
    pub updated_at: UtcTimestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollectionItemState {
    Queued,
    Running,
    Saved,
    Skipped,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectionItemStatus {
    pub session_id: CollectionSessionId,
    pub candidate_id: String,
    pub state: CollectionItemState,
    pub attempt_count: u32,
    pub reason: Option<String>,
    pub retryable: bool,
    pub requested_attachments: bool,
    pub item_id: Option<ItemId>,
    pub revision_id: Option<RevisionId>,
    pub updated_at: UtcTimestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecollectionState {
    Changed,
    Unchanged,
    Inaccessible,
    Removed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecollectionOutcome {
    pub item_id: ItemId,
    pub state: RecollectionState,
    pub previous_revision_id: RevisionId,
    pub new_revision_id: Option<RevisionId>,
    pub reason: Option<String>,
    pub checked_at: UtcTimestamp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceObservationKind {
    Capture,
    Recollection,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn common_source_metadata_is_versioned_and_validated() {
        let metadata = CommonSourceMetadata {
            title: Some("A source title".to_owned()),
            authors: vec![SourceAuthor {
                display_name: "Author".to_owned(),
                native_id: Some("author-1".to_owned()),
                locator: None,
            }],
            hierarchy: vec![SourceHierarchyNode {
                kind: Some("collection".to_owned()),
                name: "Saved".to_owned(),
                native_id: None,
                locator: None,
            }],
            media: SourceMediaMetadata {
                entries: vec![SourceMediaEntry {
                    kind: "video".to_owned(),
                    media_type: Some("video/mp4".to_owned()),
                    duration_ms: Some(1_000),
                    width: Some(1_920),
                    height: Some(1_080),
                    page_count: None,
                }],
                ..SourceMediaMetadata::default()
            },
            ..CommonSourceMetadata::default()
        };
        metadata.validate().unwrap();
        let json = serde_json::to_string(&metadata).unwrap();
        let decoded: CommonSourceMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, metadata);
    }

    #[test]
    fn legacy_candidate_fields_fill_only_missing_common_values() {
        let summary = CandidateSummary {
            candidate_id: "candidate".to_owned(),
            session_id: CollectionSessionId::new(),
            route_id: SourceRouteId("source.fixture".to_owned()),
            source_native_id: None,
            title: Some("Legacy title".to_owned()),
            source_location: None,
            hierarchy: vec!["Folder".to_owned()],
            content_type: ContentType::Document,
            source_updated_at: None,
            attachment_available: None,
            limitations: vec!["Legacy limitation".to_owned()],
            selection_capabilities: vec!["single".to_owned()],
            common_metadata: CommonSourceMetadata {
                title: Some("Typed title".to_owned()),
                ..CommonSourceMetadata::default()
            },
        };
        let common = summary.effective_common_metadata();
        assert_eq!(common.title.as_deref(), Some("Typed title"));
        assert_eq!(common.hierarchy[0].name, "Folder");
        assert_eq!(common.limitations[0].detail, "Legacy limitation");
    }

    #[test]
    fn blank_legacy_candidate_fields_do_not_create_invalid_common_metadata() {
        let summary = CandidateSummary {
            candidate_id: "candidate".to_owned(),
            session_id: CollectionSessionId::new(),
            route_id: SourceRouteId("source.fixture".to_owned()),
            source_native_id: None,
            title: Some(" ".to_owned()),
            source_location: None,
            hierarchy: vec![String::new()],
            content_type: ContentType::Document,
            source_updated_at: None,
            attachment_available: None,
            limitations: vec![" ".to_owned()],
            selection_capabilities: vec!["single".to_owned()],
            common_metadata: CommonSourceMetadata::default(),
        };

        let common = summary.effective_common_metadata();
        common.validate().unwrap();
        assert!(common.title.is_none());
        assert!(common.hierarchy.is_empty());
        assert!(common.limitations.is_empty());
        assert!(common.context.is_none());
    }
}
