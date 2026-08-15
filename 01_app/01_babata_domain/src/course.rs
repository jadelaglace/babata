use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::{DomainError, UtcTimestamp};

pub const COURSE_REGISTRATION_SCHEMA_V1: &str = "babata.course-registration/v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CourseAcceptanceState {
    PendingUserAcceptance,
    Accepted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CourseClosureState {
    Open,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MapAssignmentRole {
    Primary,
    Secondary,
    Contextual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TypedMapRelationKind {
    IntersectsWith,
    DrawsFrom,
    AppliesTo,
    PrerequisiteOf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CourseBranchCover {
    pub branch_map_node_id: String,
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CourseMapAssignment {
    pub map_node_id: String,
    pub role: MapAssignmentRole,
    pub strength: u8,
    pub confidence: u8,
    pub rationale: String,
    pub method_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CourseSemanticModule {
    pub module_id: String,
    pub semantic_id: String,
    pub chapter_id: String,
    pub assignments: Vec<CourseMapAssignment>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TypedMapRelation {
    pub from_map_node_id: String,
    pub kind: TypedMapRelationKind,
    pub to_map_node_id: String,
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CourseLensRef {
    pub sublibrary_id: String,
    pub definition_version: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CourseRegistrationDefinition {
    pub schema_version: String,
    pub course_key: String,
    pub version: u32,
    pub title: String,
    pub source: String,
    pub term: String,
    pub acceptance_state: CourseAcceptanceState,
    pub closure_state: CourseClosureState,
    pub branches: Vec<CourseBranchCover>,
    pub modules: Vec<CourseSemanticModule>,
    #[serde(default)]
    pub map_relations: Vec<TypedMapRelation>,
    pub lens: CourseLensRef,
    pub author_kind: String,
    pub author: String,
    pub created_at: UtcTimestamp,
}

impl CourseRegistrationDefinition {
    #[allow(clippy::too_many_lines)]
    pub fn validate(&self) -> Result<(), DomainError> {
        if self.schema_version != COURSE_REGISTRATION_SCHEMA_V1 {
            return invalid("course registration schema_version", &self.schema_version);
        }
        if self.version == 0 {
            return invalid("course version", &self.version.to_string());
        }
        if self.course_key.is_empty()
            || !self
                .course_key
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        {
            return invalid("course_key", &self.course_key);
        }
        non_blank("course title", &self.title)?;
        non_blank("course source", &self.source)?;
        non_blank("course term", &self.term)?;
        if !matches!(
            self.author_kind.as_str(),
            "system" | "machine" | "first_party"
        ) {
            return invalid("course author_kind", &self.author_kind);
        }
        non_blank("course author", &self.author)?;
        if self.branches.is_empty() {
            return Err(DomainError::Empty {
                field: "course branches",
            });
        }
        if self.modules.is_empty() {
            return Err(DomainError::Empty {
                field: "course modules",
            });
        }
        validate_prefixed_id(
            "lens sublibrary_id",
            &self.lens.sublibrary_id,
            "sublibrary_",
        )?;
        if self.lens.definition_version == 0 {
            return invalid("lens definition_version", "0");
        }

        let mut branch_ids = HashSet::new();
        for branch in &self.branches {
            validate_prefixed_id("covered branch", &branch.branch_map_node_id, "mapnode_")?;
            non_blank("branch cover rationale", &branch.rationale)?;
            if !branch_ids.insert(&branch.branch_map_node_id) {
                return invalid("covered branch", &branch.branch_map_node_id);
            }
        }

        let mut module_ids = HashSet::new();
        let mut semantic_ids = HashSet::new();
        for module in &self.modules {
            non_blank("course module_id", &module.module_id)?;
            non_blank("course chapter_id", &module.chapter_id)?;
            validate_prefixed_id("course semantic_id", &module.semantic_id, "semantic_")?;
            if !module_ids.insert(&module.module_id) || !semantic_ids.insert(&module.semantic_id) {
                return invalid("course module identity", &module.module_id);
            }
            if module.assignments.is_empty() {
                return Err(DomainError::Empty {
                    field: "course module assignments",
                });
            }
            let mut assigned_nodes = HashSet::new();
            for assignment in &module.assignments {
                validate_prefixed_id(
                    "course assignment map_node_id",
                    &assignment.map_node_id,
                    "mapnode_",
                )?;
                if assignment.strength > 100 || assignment.confidence > 100 {
                    return invalid(
                        "course assignment strength/confidence",
                        &format!("{}/{}", assignment.strength, assignment.confidence),
                    );
                }
                non_blank("course assignment rationale", &assignment.rationale)?;
                non_blank(
                    "course assignment method_version",
                    &assignment.method_version,
                )?;
                if !assigned_nodes.insert(&assignment.map_node_id) {
                    return invalid("course assignment map_node_id", &assignment.map_node_id);
                }
            }
        }

        let mut relations = HashSet::new();
        for relation in &self.map_relations {
            validate_prefixed_id(
                "map relation source",
                &relation.from_map_node_id,
                "mapnode_",
            )?;
            validate_prefixed_id("map relation target", &relation.to_map_node_id, "mapnode_")?;
            if relation.from_map_node_id == relation.to_map_node_id {
                return invalid("map relation", "self relation");
            }
            non_blank("map relation rationale", &relation.rationale)?;
            if !relations.insert((
                &relation.from_map_node_id,
                relation.kind,
                &relation.to_map_node_id,
            )) {
                return invalid("map relation", &relation.from_map_node_id);
            }
        }
        Ok(())
    }

    pub fn course_ref(&self) -> String {
        format!("course:{}@{}", self.course_key, self.version)
    }
}

fn non_blank(field: &'static str, value: &str) -> Result<(), DomainError> {
    if value.trim().is_empty() {
        Err(DomainError::Empty { field })
    } else {
        Ok(())
    }
}

fn validate_prefixed_id(field: &'static str, value: &str, prefix: &str) -> Result<(), DomainError> {
    if value.starts_with(prefix) && value.len() == prefix.len() + 26 {
        Ok(())
    } else {
        invalid(field, value)
    }
}

fn invalid<T>(field: &'static str, value: &str) -> Result<T, DomainError> {
    Err(DomainError::Invalid {
        field,
        value: value.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn definition() -> CourseRegistrationDefinition {
        CourseRegistrationDefinition {
            schema_version: COURSE_REGISTRATION_SCHEMA_V1.to_owned(),
            course_key: "decision-accounting".to_owned(),
            version: 1,
            title: "Decision Accounting".to_owned(),
            source: "MBA".to_owned(),
            term: "2025-spring".to_owned(),
            acceptance_state: CourseAcceptanceState::PendingUserAcceptance,
            closure_state: CourseClosureState::Open,
            branches: vec![CourseBranchCover {
                branch_map_node_id: "mapnode_01J00000000000000000000000".to_owned(),
                rationale: "Course coverage".to_owned(),
            }],
            modules: vec![CourseSemanticModule {
                module_id: "1".to_owned(),
                semantic_id: "semantic_01J00000000000000000000000".to_owned(),
                chapter_id: "01".to_owned(),
                assignments: vec![CourseMapAssignment {
                    map_node_id: "mapnode_01J00000000000000000000000".to_owned(),
                    role: MapAssignmentRole::Primary,
                    strength: 80,
                    confidence: 70,
                    rationale: "Primary coverage".to_owned(),
                    method_version: "course-map/v1".to_owned(),
                }],
            }],
            map_relations: Vec::new(),
            lens: CourseLensRef {
                sublibrary_id: "sublibrary_01J00000000000000000000000".to_owned(),
                definition_version: 1,
            },
            author_kind: "machine".to_owned(),
            author: "babata".to_owned(),
            created_at: UtcTimestamp::parse("2026-08-15T00:00:00Z").unwrap(),
        }
    }

    #[test]
    fn course_registration_keeps_strength_and_confidence_independent() {
        let definition = definition();
        assert!(definition.validate().is_ok());
        assert_eq!(definition.modules[0].assignments[0].strength, 80);
        assert_eq!(definition.modules[0].assignments[0].confidence, 70);
    }

    #[test]
    fn course_registration_rejects_duplicate_assignment_without_sum_rule() {
        let mut duplicate_case = definition();
        let duplicate_assignment = duplicate_case.modules[0].assignments[0].clone();
        duplicate_case.modules[0]
            .assignments
            .push(duplicate_assignment);
        assert!(duplicate_case.validate().is_err());

        let mut independent_case = definition();
        independent_case.modules[0].assignments[0].strength = 100;
        independent_case.modules[0]
            .assignments
            .push(CourseMapAssignment {
                map_node_id: "mapnode_01J00000000000000000000001".to_owned(),
                role: MapAssignmentRole::Secondary,
                strength: 80,
                confidence: 60,
                rationale: "Independent secondary dimension".to_owned(),
                method_version: "course-map/v1".to_owned(),
            });
        assert!(independent_case.validate().is_ok());
    }
}
