use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::{
    DomainError, ItemId, QueryFilter, RecordSummary, RevisionId, Sha256, SublibraryId, UtcTimestamp,
};

pub const SUBLIBRARY_SCHEMA_VERSION: &str = "babata.sublibrary/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SublibraryOrganisationRule {
    ManualFirst,
    WeightedScoreDescending,
    EventNewest,
    SourceThenTitle,
    MapThenTitle,
    Title,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SublibraryDefinitionInput {
    pub title: String,
    pub purpose: String,
    pub selection: QueryFilter,
    pub manual_include: Vec<String>,
    pub manual_exclude: Vec<String>,
    pub organisation_rules: Vec<SublibraryOrganisationRule>,
    pub include_unreviewed: bool,
}

impl Default for SublibraryDefinitionInput {
    fn default() -> Self {
        Self {
            title: String::new(),
            purpose: String::new(),
            selection: QueryFilter::default(),
            manual_include: Vec::new(),
            manual_exclude: Vec::new(),
            organisation_rules: vec![SublibraryOrganisationRule::ManualFirst],
            include_unreviewed: false,
        }
    }
}

impl SublibraryDefinitionInput {
    pub fn validate(&self) -> Result<(), DomainError> {
        require_text("sublibrary title", &self.title)?;
        require_text("sublibrary purpose", &self.purpose)?;
        validate_record_ids("manual_include", &self.manual_include)?;
        validate_record_ids("manual_exclude", &self.manual_exclude)?;
        let includes = self.manual_include.iter().collect::<HashSet<_>>();
        if self
            .manual_exclude
            .iter()
            .any(|record_id| includes.contains(record_id))
        {
            return Err(DomainError::Invalid {
                field: "manual_include/manual_exclude",
                value: "the same record cannot be manually included and excluded".to_owned(),
            });
        }
        if self.organisation_rules.is_empty() {
            return Err(DomainError::Empty {
                field: "organisation_rules",
            });
        }
        let unique_rules = self.organisation_rules.iter().collect::<HashSet<_>>();
        if unique_rules.len() != self.organisation_rules.len() {
            return Err(DomainError::Invalid {
                field: "organisation_rules",
                value: "duplicate organisation rules".to_owned(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SublibraryAuthorityRef {
    pub item_id: ItemId,
    pub revision_id: RevisionId,
    pub text_sha256: Sha256,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SublibraryDefinition {
    pub schema_version: String,
    pub id: SublibraryId,
    pub version: u32,
    #[serde(flatten)]
    pub definition: SublibraryDefinitionInput,
    pub author: String,
    pub created_at: UtcTimestamp,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub authority: Option<SublibraryAuthorityRef>,
}

impl SublibraryDefinition {
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.schema_version != SUBLIBRARY_SCHEMA_VERSION {
            return Err(DomainError::Invalid {
                field: "schema_version",
                value: self.schema_version.clone(),
            });
        }
        if self.version == 0 {
            return Err(DomainError::Invalid {
                field: "version",
                value: self.version.to_string(),
            });
        }
        require_text("author", &self.author)?;
        self.definition.validate()
    }

    pub fn canonical_json(&self) -> Result<String, serde_json::Error> {
        let mut canonical = self.clone();
        canonical.authority = None;
        serde_json::to_string_pretty(&canonical)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SublibraryMember {
    pub position: u32,
    pub record: RecordSummary,
    pub input_sha256: Sha256,
    pub inclusion_reasons: Vec<String>,
    pub organisation_keys: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SublibraryExclusion {
    pub record_id: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SublibraryMaterializationDocument {
    pub definition: SublibraryDefinition,
    pub definition_sha256: Sha256,
    pub projection_fingerprint: String,
    pub built_at: UtcTimestamp,
    pub members: Vec<SublibraryMember>,
    pub exclusions: Vec<SublibraryExclusion>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SublibraryMaterializationState {
    Succeeded,
    Verified,
    Deleted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SublibraryMaterialization {
    pub sublibrary_id: SublibraryId,
    pub definition_version: u32,
    pub state: SublibraryMaterializationState,
    pub member_count: u64,
    pub definition_sha256: Sha256,
    pub projection_fingerprint: String,
    pub output_sha256: Sha256,
    pub materialization_path: String,
    pub manifest_path: String,
    pub built_at: UtcTimestamp,
}

fn require_text(field: &'static str, value: &str) -> Result<(), DomainError> {
    if value.trim().is_empty() {
        Err(DomainError::Empty { field })
    } else {
        Ok(())
    }
}

fn validate_record_ids(field: &'static str, values: &[String]) -> Result<(), DomainError> {
    let mut unique = HashSet::new();
    for value in values {
        if !(value.starts_with("item:") || value.starts_with("semantic:"))
            || value.split_once(':').is_none_or(|(_, id)| id.is_empty())
        {
            return Err(DomainError::Invalid {
                field,
                value: value.clone(),
            });
        }
        if !unique.insert(value) {
            return Err(DomainError::Invalid {
                field,
                value: format!("duplicate record ID {value}"),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_scope_rejects_overlap_and_duplicate_rules() {
        let input = SublibraryDefinitionInput {
            title: "Project".to_owned(),
            purpose: "Keep the active project evidence together".to_owned(),
            manual_include: vec!["item:item_01J00000000000000000000000".to_owned()],
            manual_exclude: vec!["item:item_01J00000000000000000000000".to_owned()],
            organisation_rules: vec![
                SublibraryOrganisationRule::Title,
                SublibraryOrganisationRule::Title,
            ],
            ..SublibraryDefinitionInput::default()
        };
        assert!(input.validate().is_err());
    }
}
