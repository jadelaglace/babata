use serde::{Deserialize, Serialize};

use crate::{DerivativeId, DomainError, ItemId, RevisionId, UtcTimestamp};

pub const SEMANTIC_CANDIDATE_SCHEMA_V1: &str = "p6-semantic-candidate/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeRealm {
    KnowledgeMap,
    KnowledgeAndCases,
    CognitiveTrail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KnowledgeKind {
    MapDirection,
    Knowledge,
    Case,
    Log,
    Insight,
}

impl KnowledgeKind {
    pub const fn realm(self) -> KnowledgeRealm {
        match self {
            Self::MapDirection => KnowledgeRealm::KnowledgeMap,
            Self::Knowledge | Self::Case => KnowledgeRealm::KnowledgeAndCases,
            Self::Log | Self::Insight => KnowledgeRealm::CognitiveTrail,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MapNodeLevel {
    Foundation,
    Discipline,
    Branch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MapNodeLifecycle {
    Active,
    Inactive,
    Merged,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelevanceTargetKind {
    MapNode,
    Semantic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogScale {
    LongTerm,
    MediumTerm,
    ShortTerm,
    Realtime,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InsightMaturity {
    Spark,
    Framework,
    Mature,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DenseExpressionKind {
    MindMap,
    Mermaid,
    Model,
    Formula,
    Checklist,
    Process,
    Outline,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SemanticPayload {
    MapDirection {
        description: String,
    },
    Knowledge {
        statement: String,
        details: String,
    },
    Case {
        scenario: String,
        process: String,
        result: String,
        analysis: String,
    },
    Log {
        scale: LogScale,
        occurred_at: UtcTimestamp,
        body: String,
    },
    Insight {
        maturity: InsightMaturity,
        body: String,
    },
}

impl SemanticPayload {
    pub const fn knowledge_kind(&self) -> KnowledgeKind {
        match self {
            Self::MapDirection { .. } => KnowledgeKind::MapDirection,
            Self::Knowledge { .. } => KnowledgeKind::Knowledge,
            Self::Case { .. } => KnowledgeKind::Case,
            Self::Log { .. } => KnowledgeKind::Log,
            Self::Insight { .. } => KnowledgeKind::Insight,
        }
    }

    fn validate(&self) -> Result<(), DomainError> {
        match self {
            Self::MapDirection { description } => non_blank(description, "map description"),
            Self::Knowledge { statement, details } => {
                non_blank(statement, "knowledge statement")?;
                non_blank(details, "knowledge details")
            }
            Self::Case {
                scenario,
                process,
                result,
                analysis,
            } => {
                non_blank(scenario, "case scenario")?;
                non_blank(process, "case process")?;
                non_blank(result, "case result")?;
                non_blank(analysis, "case analysis")
            }
            Self::Log { body, .. } => non_blank(body, "log body"),
            Self::Insight { body, .. } => non_blank(body, "insight body"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DenseExpressionCandidate {
    pub kind: DenseExpressionKind,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelevanceComponents {
    pub interest: u8,
    pub strategy: u8,
    pub consensus: u8,
    pub rationale: String,
}

impl RelevanceComponents {
    pub fn validate(&self) -> Result<(), DomainError> {
        for (field, value) in [
            ("interest", self.interest),
            ("strategy", self.strategy),
            ("consensus", self.consensus),
        ] {
            if value > 100 {
                return Err(DomainError::Invalid {
                    field,
                    value: value.to_string(),
                });
            }
        }
        non_blank(&self.rationale, "score rationale")
    }

    pub fn weighted_score(&self, profile: &ScoreProfile) -> u16 {
        let weighted = u32::from(self.interest) * u32::from(profile.interest_weight)
            + u32::from(self.strategy) * u32::from(profile.strategy_weight)
            + u32::from(self.consensus) * u32::from(profile.consensus_weight);
        weighted as u16
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScoreProfile {
    pub profile_id: String,
    pub ordinal: u32,
    pub interest_weight: u8,
    pub strategy_weight: u8,
    pub consensus_weight: u8,
    pub rationale: String,
    pub author_kind: String,
    pub author: String,
    pub created_at: UtcTimestamp,
}

impl ScoreProfile {
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.ordinal == 0
            || u16::from(self.interest_weight)
                + u16::from(self.strategy_weight)
                + u16::from(self.consensus_weight)
                != 100
        {
            return Err(DomainError::Invalid {
                field: "score profile weights",
                value: format!(
                    "{}/{}/{}",
                    self.interest_weight, self.strategy_weight, self.consensus_weight
                ),
            });
        }
        non_blank(&self.profile_id, "profile id")?;
        non_blank(&self.rationale, "profile rationale")?;
        if !matches!(
            self.author_kind.as_str(),
            "system" | "machine" | "first_party"
        ) {
            return Err(DomainError::Invalid {
                field: "profile author kind",
                value: self.author_kind.clone(),
            });
        }
        non_blank(&self.author, "profile author")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MapNodeCandidate {
    pub local_ref: String,
    pub level: MapNodeLevel,
    pub name: String,
    pub parent_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticCandidate {
    pub local_ref: String,
    pub title: String,
    pub payload: SemanticPayload,
    pub map_node_refs: Vec<String>,
    pub tags: Vec<String>,
    pub dense_expressions: Vec<DenseExpressionCandidate>,
    pub relevance: RelevanceComponents,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CandidateRelation {
    pub from_ref: String,
    pub kind: String,
    pub to_ref: String,
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DerivativeEvidence {
    pub derivative_id: DerivativeId,
    pub output_sha256: crate::Sha256,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticCandidatePackage {
    pub schema_version: String,
    pub source_item_id: ItemId,
    pub source_revision_id: RevisionId,
    pub evidence_derivatives: Vec<DerivativeEvidence>,
    pub provider: String,
    pub model: String,
    pub model_version: String,
    pub prompt_version: String,
    pub generated_at: UtcTimestamp,
    pub map_nodes: Vec<MapNodeCandidate>,
    pub entries: Vec<SemanticCandidate>,
    pub relations: Vec<CandidateRelation>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticCandidateBody {
    pub map_nodes: Vec<MapNodeCandidate>,
    pub entries: Vec<SemanticCandidate>,
    pub relations: Vec<CandidateRelation>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstPartySemanticDefinition {
    pub title: String,
    pub payload: SemanticPayload,
    pub map_node_refs: Vec<String>,
    pub tags: Vec<String>,
    pub dense_expressions: Vec<DenseExpressionCandidate>,
    pub relevance: RelevanceComponents,
    pub relations: Vec<FirstPartySemanticRelation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FirstPartySemanticRelation {
    pub kind: String,
    pub to_semantic_id: String,
    pub evidence: String,
}

impl FirstPartySemanticDefinition {
    pub fn validate(&self) -> Result<(), DomainError> {
        non_blank(&self.title, "first-party semantic title")?;
        self.payload.validate()?;
        self.relevance.validate()?;
        if self.map_node_refs.is_empty() {
            return Err(DomainError::Empty {
                field: "first-party map assignments",
            });
        }
        for value in &self.map_node_refs {
            non_blank(value, "first-party map ref")?;
        }
        for value in &self.tags {
            non_blank(value, "first-party tag")?;
        }
        for expression in &self.dense_expressions {
            non_blank(&expression.content, "first-party dense expression")?;
        }
        for relation in &self.relations {
            non_blank(&relation.kind, "first-party relation kind")?;
            non_blank(&relation.to_semantic_id, "first-party relation target")?;
            non_blank(&relation.evidence, "first-party relation evidence")?;
        }
        Ok(())
    }
}

impl SemanticCandidatePackage {
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.schema_version != SEMANTIC_CANDIDATE_SCHEMA_V1 {
            return Err(DomainError::Invalid {
                field: "semantic candidate schema_version",
                value: self.schema_version.clone(),
            });
        }
        for (field, value) in [
            ("provider", &self.provider),
            ("model", &self.model),
            ("model_version", &self.model_version),
            ("prompt_version", &self.prompt_version),
        ] {
            non_blank(value, field)?;
        }
        if self.evidence_derivatives.is_empty() {
            return Err(DomainError::Empty {
                field: "semantic evidence derivatives",
            });
        }
        let mut evidence_ids = std::collections::BTreeSet::new();
        for evidence in &self.evidence_derivatives {
            if !evidence_ids.insert(evidence.derivative_id.to_string()) {
                return Err(DomainError::Invalid {
                    field: "semantic evidence derivatives",
                    value: evidence.derivative_id.to_string(),
                });
            }
        }
        if self.entries.is_empty() {
            return Err(DomainError::Empty {
                field: "semantic entries",
            });
        }

        let refs = self.validated_map_refs()?;

        let mut entry_refs = std::collections::BTreeSet::new();
        for entry in &self.entries {
            non_blank(&entry.local_ref, "entry local_ref")?;
            non_blank(&entry.title, "entry title")?;
            entry.payload.validate()?;
            entry.relevance.validate()?;
            if entry.map_node_refs.is_empty()
                || entry.map_node_refs.iter().any(|node| !refs.contains(node))
                || !entry_refs.insert(entry.local_ref.clone())
            {
                return Err(DomainError::Invalid {
                    field: "semantic entry refs",
                    value: entry.local_ref.clone(),
                });
            }
            for tag in &entry.tags {
                non_blank(tag, "tag")?;
            }
            for expression in &entry.dense_expressions {
                non_blank(&expression.content, "dense expression")?;
            }
        }
        for relation in &self.relations {
            if !entry_refs.contains(&relation.from_ref)
                || !entry_refs.contains(&relation.to_ref)
                || relation.from_ref == relation.to_ref
            {
                return Err(DomainError::Invalid {
                    field: "candidate relation endpoints",
                    value: format!("{} -> {}", relation.from_ref, relation.to_ref),
                });
            }
            non_blank(&relation.kind, "relation kind")?;
            non_blank(&relation.evidence, "relation evidence")?;
        }
        Ok(())
    }

    fn validated_map_refs(&self) -> Result<std::collections::BTreeSet<String>, DomainError> {
        let mut refs = std::collections::BTreeSet::new();
        let mut map_levels = std::collections::BTreeMap::new();
        for root in [
            "foundation:time",
            "foundation:space",
            "foundation:matter",
            "foundation:consciousness",
        ] {
            refs.insert(root.to_owned());
            map_levels.insert(root.to_owned(), MapNodeLevel::Foundation);
        }
        for node in &self.map_nodes {
            if node.level == MapNodeLevel::Foundation {
                return Err(DomainError::Invalid {
                    field: "candidate map node level",
                    value: "foundation roots are fixed by the worldview map".to_owned(),
                });
            }
            non_blank(&node.local_ref, "map node local_ref")?;
            non_blank(&node.name, "map node name")?;
            if node.parent_refs.is_empty() || !refs.insert(node.local_ref.clone()) {
                return Err(DomainError::Invalid {
                    field: "map node local_ref",
                    value: node.local_ref.clone(),
                });
            }
            map_levels.insert(node.local_ref.clone(), node.level);
        }
        for node in &self.map_nodes {
            let invalid_parent = node.parent_refs.iter().find(|parent| {
                map_levels.get(*parent).is_none_or(|parent_level| {
                    !matches!(
                        (*parent_level, node.level),
                        (MapNodeLevel::Foundation, MapNodeLevel::Discipline)
                            | (MapNodeLevel::Discipline, MapNodeLevel::Branch)
                    )
                })
            });
            if let Some(invalid_parent) = invalid_parent {
                return Err(DomainError::Invalid {
                    field: "map node parent_refs",
                    value: format!("{} -> {}", invalid_parent, node.local_ref),
                });
            }
        }
        Ok(refs)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuggestionDecisionKind {
    Accept,
    Modify,
    Reject,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SuggestionDecision {
    pub suggestion_id: String,
    pub decision: SuggestionDecisionKind,
    pub reason: Option<String>,
    pub first_party_item_id: Option<ItemId>,
}

fn non_blank(value: &str, field: &'static str) -> Result<(), DomainError> {
    if value.trim().is_empty() {
        Err(DomainError::Empty { field })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semantic_kinds_remain_inside_their_realms() {
        assert_eq!(
            KnowledgeKind::MapDirection.realm(),
            KnowledgeRealm::KnowledgeMap
        );
        assert_eq!(
            KnowledgeKind::Knowledge.realm(),
            KnowledgeRealm::KnowledgeAndCases
        );
        assert_eq!(
            KnowledgeKind::Case.realm(),
            KnowledgeRealm::KnowledgeAndCases
        );
        assert_eq!(KnowledgeKind::Log.realm(), KnowledgeRealm::CognitiveTrail);
        assert_eq!(
            KnowledgeKind::Insight.realm(),
            KnowledgeRealm::CognitiveTrail
        );
    }

    #[test]
    fn default_weights_compute_an_explainable_score() {
        let profile = ScoreProfile {
            profile_id: "default".to_owned(),
            ordinal: 1,
            interest_weight: 40,
            strategy_weight: 35,
            consensus_weight: 25,
            rationale: "P6 baseline".to_owned(),
            author_kind: "system".to_owned(),
            author: "babata".to_owned(),
            created_at: UtcTimestamp::parse("2026-01-01T00:00:00Z").unwrap(),
        };
        profile.validate().unwrap();
        let score = RelevanceComponents {
            interest: 80,
            strategy: 60,
            consensus: 40,
            rationale: "fixture".to_owned(),
        };
        assert_eq!(score.weighted_score(&profile), 6300);
    }

    #[test]
    fn semantic_package_rejects_a_branch_attached_directly_to_a_foundation() {
        let package = SemanticCandidatePackage {
            schema_version: SEMANTIC_CANDIDATE_SCHEMA_V1.to_owned(),
            source_item_id: ItemId::new(),
            source_revision_id: RevisionId::new(),
            evidence_derivatives: vec![DerivativeEvidence {
                derivative_id: DerivativeId::new(),
                output_sha256: crate::Sha256::of_bytes(b"evidence"),
            }],
            provider: "fixture".to_owned(),
            model: "fixture".to_owned(),
            model_version: "1".to_owned(),
            prompt_version: "1".to_owned(),
            generated_at: UtcTimestamp::parse("2026-07-21T00:00:00Z").unwrap(),
            map_nodes: vec![MapNodeCandidate {
                local_ref: "node:invalid-branch".to_owned(),
                level: MapNodeLevel::Branch,
                name: "invalid branch".to_owned(),
                parent_refs: vec!["foundation:time".to_owned()],
            }],
            entries: vec![SemanticCandidate {
                local_ref: "entry:knowledge".to_owned(),
                title: "knowledge".to_owned(),
                payload: SemanticPayload::Knowledge {
                    statement: "statement".to_owned(),
                    details: "details".to_owned(),
                },
                map_node_refs: vec!["foundation:time".to_owned()],
                tags: Vec::new(),
                dense_expressions: Vec::new(),
                relevance: RelevanceComponents {
                    interest: 1,
                    strategy: 1,
                    consensus: 1,
                    rationale: "fixture".to_owned(),
                },
            }],
            relations: Vec::new(),
            limitations: Vec::new(),
        };

        assert!(package.validate().is_err());
    }
}
