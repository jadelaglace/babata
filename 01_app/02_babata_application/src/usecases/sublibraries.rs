use std::{
    cmp::Ordering,
    collections::{BTreeSet, HashMap, HashSet},
};

use babata_domain::{
    Metadata, PageCursor, SUBLIBRARY_SCHEMA_VERSION, SearchRecordDetail, Sha256,
    SublibraryAuthorityRef, SublibraryDefinition, SublibraryExclusion, SublibraryId,
    SublibraryMaterialization, SublibraryMaterializationDocument, SublibraryMember,
    SublibraryOrganisationRule,
};

use crate::{
    ApplicationError, CreateNoteCommand, CreateSublibraryCommand, ReviseCommand,
    ReviseSublibraryCommand, SearchQuery, WorkspaceService,
    ports::{
        AssetStorePort, ClockPort, RawRepositoryPort, ReadProjectionPort, SublibraryDefinitionPort,
        SublibraryMaterializerPort,
    },
};

pub struct SublibraryService<R, A, C, D, P, M> {
    workspace: WorkspaceService<R, A, C>,
    clock: C,
    definitions: D,
    projection: P,
    materializer: M,
}

impl<R, A, C, D, P, M> SublibraryService<R, A, C, D, P, M>
where
    R: RawRepositoryPort,
    A: AssetStorePort,
    C: ClockPort + Clone,
    D: SublibraryDefinitionPort,
    P: ReadProjectionPort,
    M: SublibraryMaterializerPort,
{
    pub fn new(
        repository: R,
        assets: A,
        clock: C,
        definitions: D,
        projection: P,
        materializer: M,
    ) -> Self {
        Self {
            workspace: WorkspaceService::new(repository, assets, clock.clone()),
            clock,
            definitions,
            projection,
            materializer,
        }
    }

    pub fn create(
        &self,
        command: CreateSublibraryCommand,
    ) -> Result<SublibraryDefinition, ApplicationError> {
        command.definition.validate()?;
        if command.author.trim().is_empty() {
            return Err(ApplicationError::Integrity(
                "sublibrary author is required".to_owned(),
            ));
        }
        let mut definition = SublibraryDefinition {
            schema_version: SUBLIBRARY_SCHEMA_VERSION.to_owned(),
            id: SublibraryId::new(),
            version: 1,
            definition: command.definition,
            author: command.author,
            created_at: self.clock.now(),
            authority: None,
        };
        definition.validate()?;
        let body = definition.canonical_json().map_err(json_error)?;
        let metadata = definition_metadata(&definition)?;
        let outcome = self.workspace.create(CreateNoteCommand {
            text: body.clone(),
            path: None,
            context: Some("Babata versioned sublibrary definition".to_owned()),
            metadata,
        })?;
        definition.authority = Some(SublibraryAuthorityRef {
            item_id: outcome.item_id,
            revision_id: outcome.revision_id,
            text_sha256: Sha256::of_bytes(body.as_bytes()),
        });
        Ok(definition)
    }

    pub fn revise(
        &self,
        command: ReviseSublibraryCommand,
    ) -> Result<SublibraryDefinition, ApplicationError> {
        command.definition.validate()?;
        let current = self
            .definitions
            .find(&command.sublibrary_id, None)?
            .ok_or_else(|| ApplicationError::NotFound(command.sublibrary_id.to_string()))?;
        if current.version != command.expected_version {
            return Err(ApplicationError::Conflict(format!(
                "sublibrary {} is at version {}, not expected version {}",
                current.id, current.version, command.expected_version
            )));
        }
        let authority = current.authority.ok_or_else(|| {
            ApplicationError::Integrity(
                "sublibrary definition has no C0 authority reference".to_owned(),
            )
        })?;
        let mut definition = SublibraryDefinition {
            schema_version: SUBLIBRARY_SCHEMA_VERSION.to_owned(),
            id: current.id,
            version: current.version.checked_add(1).ok_or_else(|| {
                ApplicationError::Integrity("sublibrary version overflow".to_owned())
            })?,
            definition: command.definition,
            author: command.author,
            created_at: self.clock.now(),
            authority: None,
        };
        definition.validate()?;
        let body = definition.canonical_json().map_err(json_error)?;
        let outcome = self.workspace.revise(ReviseCommand {
            parent: authority.revision_id,
            text: body.clone(),
            path: None,
            note: Some(format!(
                "sublibrary definition version {}",
                definition.version
            )),
            metadata: definition_metadata(&definition)?,
        })?;
        definition.authority = Some(SublibraryAuthorityRef {
            item_id: outcome.item_id,
            revision_id: outcome.revision_id,
            text_sha256: Sha256::of_bytes(body.as_bytes()),
        });
        Ok(definition)
    }

    pub fn list(&self) -> Result<Vec<SublibraryDefinition>, ApplicationError> {
        self.definitions.list_latest()
    }

    pub fn versions(
        &self,
        sublibrary_id: &SublibraryId,
    ) -> Result<Vec<SublibraryDefinition>, ApplicationError> {
        self.definitions.list_versions(sublibrary_id)
    }

    pub fn show(
        &self,
        sublibrary_id: &SublibraryId,
        version: Option<u32>,
    ) -> Result<SublibraryDefinition, ApplicationError> {
        self.definitions
            .find(sublibrary_id, version)?
            .ok_or_else(|| ApplicationError::NotFound(sublibrary_id.to_string()))
    }

    pub fn materialize(
        &self,
        sublibrary_id: &SublibraryId,
        version: Option<u32>,
    ) -> Result<SublibraryMaterialization, ApplicationError> {
        let definition = self.show(sublibrary_id, version)?;
        let document = resolve_definition(&self.projection, definition, self.clock.now())?;
        self.materializer.build(&document)
    }

    pub fn materialization_status(
        &self,
        sublibrary_id: &SublibraryId,
        version: u32,
    ) -> Result<SublibraryMaterialization, ApplicationError> {
        self.materializer.status(sublibrary_id, version)
    }

    pub fn verify_materialization(
        &self,
        sublibrary_id: &SublibraryId,
        version: u32,
    ) -> Result<SublibraryMaterialization, ApplicationError> {
        let existing = self.materializer.status(sublibrary_id, version)?;
        let definition = self.show(sublibrary_id, Some(version))?;
        let document = resolve_definition(&self.projection, definition, existing.built_at)?;
        self.materializer.verify(&document)
    }

    pub fn delete_materialization(
        &self,
        sublibrary_id: &SublibraryId,
        version: u32,
    ) -> Result<SublibraryMaterialization, ApplicationError> {
        self.materializer.delete(sublibrary_id, version)
    }
}

#[allow(clippy::too_many_lines)]
pub(crate) fn resolve_definition<P: ReadProjectionPort>(
    projection: &P,
    definition: SublibraryDefinition,
    built_at: babata_domain::UtcTimestamp,
) -> Result<SublibraryMaterializationDocument, ApplicationError> {
    let definition_json = definition.canonical_json().map_err(json_error)?;
    let definition_sha256 = Sha256::of_bytes(definition_json.as_bytes());
    let fingerprint = projection.status()?.source_fingerprint.ok_or_else(|| {
        ApplicationError::Conflict(
            "search projection must be built before materializing a sublibrary".to_owned(),
        )
    })?;
    let mut query = definition.definition.selection.clone();
    query.limit = 200;
    let mut cursor: Option<PageCursor> = None;
    let mut records = HashMap::new();
    let mut reasons: HashMap<String, Vec<String>> = HashMap::new();
    loop {
        let page = projection.search(SearchQuery {
            filter: query.clone(),
            cursor: cursor.clone(),
        })?;
        for record in page.records {
            reasons
                .entry(record.record_id.clone())
                .or_default()
                .push("selection_rule".to_owned());
            records.insert(record.record_id.clone(), record);
        }
        cursor = page.next_cursor;
        if cursor.is_none() {
            break;
        }
    }
    for record_id in &definition.definition.manual_include {
        let detail = projection.show(record_id)?;
        reasons
            .entry(record_id.clone())
            .or_default()
            .push("manual_include".to_owned());
        records.insert(record_id.clone(), detail.record);
    }

    let manual = definition
        .definition
        .manual_include
        .iter()
        .cloned()
        .collect::<HashSet<_>>();
    let mut exclusions = definition
        .definition
        .manual_exclude
        .iter()
        .map(|record_id| SublibraryExclusion {
            record_id: record_id.clone(),
            reason: "manual_exclude".to_owned(),
        })
        .collect::<Vec<_>>();
    for record_id in &definition.definition.manual_exclude {
        records.remove(record_id);
    }
    if !definition.definition.include_unreviewed {
        let unreviewed = records
            .values()
            .filter(|record| {
                record.origin_kind == "machine"
                    && record.review_state.as_deref() == Some("unreviewed")
            })
            .map(|record| record.record_id.clone())
            .collect::<Vec<_>>();
        for record_id in unreviewed {
            records.remove(&record_id);
            exclusions.push(SublibraryExclusion {
                record_id,
                reason: "unreviewed_excluded_by_definition".to_owned(),
            });
        }
    }

    let mut resolved = records.into_values().collect::<Vec<_>>();
    resolved.sort_by(|left, right| {
        compare_records(
            left,
            right,
            &definition.definition.organisation_rules,
            &manual,
        )
    });
    let mut limitations = BTreeSet::new();
    let mut members = Vec::with_capacity(resolved.len());
    for (index, record) in resolved.into_iter().enumerate() {
        for limitation in &record.limitations {
            limitations.insert(format!("{}: {limitation}", record.record_id));
        }
        let detail = projection.show(&record.record_id)?;
        let bytes = serde_json::to_vec(&detail).map_err(json_error)?;
        members.push(SublibraryMember {
            position: u32::try_from(index + 1).map_err(|_| {
                ApplicationError::Integrity("too many sublibrary members".to_owned())
            })?,
            input_sha256: Sha256::of_bytes(&bytes),
            inclusion_reasons: reasons.remove(&record.record_id).unwrap_or_default(),
            organisation_keys: organisation_keys(
                &record,
                &definition.definition.organisation_rules,
                &manual,
            ),
            record,
        });
    }
    exclusions.sort_by(|left, right| left.record_id.cmp(&right.record_id));
    Ok(SublibraryMaterializationDocument {
        definition,
        definition_sha256,
        projection_fingerprint: fingerprint,
        built_at,
        members,
        exclusions,
        limitations: limitations.into_iter().collect(),
    })
}

fn compare_records(
    left: &babata_domain::RecordSummary,
    right: &babata_domain::RecordSummary,
    rules: &[SublibraryOrganisationRule],
    manual: &HashSet<String>,
) -> Ordering {
    for rule in rules {
        let ordering = match rule {
            SublibraryOrganisationRule::ManualFirst => manual
                .contains(&right.record_id)
                .cmp(&manual.contains(&left.record_id)),
            SublibraryOrganisationRule::WeightedScoreDescending => right
                .score
                .as_ref()
                .map(|score| score.weighted_score)
                .unwrap_or_default()
                .cmp(
                    &left
                        .score
                        .as_ref()
                        .map(|score| score.weighted_score)
                        .unwrap_or_default(),
                ),
            SublibraryOrganisationRule::EventNewest => {
                right.event_at.as_str().cmp(left.event_at.as_str())
            }
            SublibraryOrganisationRule::SourceThenTitle => left
                .provider
                .cmp(&right.provider)
                .then_with(|| left.title.cmp(&right.title)),
            SublibraryOrganisationRule::MapThenTitle => left
                .map_nodes
                .first()
                .map_or("", |node| node.name.as_str())
                .cmp(
                    right
                        .map_nodes
                        .first()
                        .map_or("", |node| node.name.as_str()),
                )
                .then_with(|| left.title.cmp(&right.title)),
            SublibraryOrganisationRule::Title => left.title.cmp(&right.title),
        };
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    left.record_id.cmp(&right.record_id)
}

fn organisation_keys(
    record: &babata_domain::RecordSummary,
    rules: &[SublibraryOrganisationRule],
    manual: &HashSet<String>,
) -> Vec<String> {
    rules
        .iter()
        .map(|rule| match rule {
            SublibraryOrganisationRule::ManualFirst => {
                format!("manual:{}", manual.contains(&record.record_id))
            }
            SublibraryOrganisationRule::WeightedScoreDescending => format!(
                "weighted_score:{}",
                record
                    .score
                    .as_ref()
                    .map(|score| score.weighted_score)
                    .unwrap_or_default()
            ),
            SublibraryOrganisationRule::EventNewest => {
                format!("event_at:{}", record.event_at.as_str())
            }
            SublibraryOrganisationRule::SourceThenTitle => {
                format!("source:{};title:{}", record.provider, record.title)
            }
            SublibraryOrganisationRule::MapThenTitle => format!(
                "map:{};title:{}",
                record
                    .map_nodes
                    .first()
                    .map_or("unassigned", |node| node.name.as_str()),
                record.title
            ),
            SublibraryOrganisationRule::Title => format!("title:{}", record.title),
        })
        .collect()
}

fn definition_metadata(definition: &SublibraryDefinition) -> Result<Metadata, ApplicationError> {
    Metadata::parse(
        &serde_json::json!({
            "babata_kind": "sublibrary_definition",
            "sublibrary_id": definition.id,
            "sublibrary_version": definition.version,
            "schema": SUBLIBRARY_SCHEMA_VERSION,
        })
        .to_string(),
    )
    .map_err(ApplicationError::from)
}

fn json_error(error: serde_json::Error) -> ApplicationError {
    ApplicationError::Integrity(error.to_string())
}

pub(crate) fn resolve_record_details<P: ReadProjectionPort>(
    projection: &P,
    record_ids: &[String],
) -> Result<Vec<SearchRecordDetail>, ApplicationError> {
    record_ids
        .iter()
        .map(|record_id| projection.show(record_id))
        .collect()
}

#[cfg(test)]
mod tests {
    use babata_domain::{
        ContentType, JudgmentStatus, KnowledgeKind, KnowledgeRealm, ProjectionStatus,
        SearchRecordKind, SourceId, SourceKind, UtcTimestamp,
    };

    use crate::{ProjectionOperationOutcome, SearchPage, SurfaceQuery};

    use super::*;

    #[derive(Clone)]
    struct FixtureProjection {
        records: Vec<babata_domain::RecordSummary>,
    }

    impl ReadProjectionPort for FixtureProjection {
        fn rebuild(&self) -> Result<ProjectionOperationOutcome, ApplicationError> {
            unreachable!()
        }
        fn delete(&self) -> Result<ProjectionOperationOutcome, ApplicationError> {
            unreachable!()
        }
        fn search(&self, _query: SearchQuery) -> Result<SearchPage, ApplicationError> {
            Ok(SearchPage {
                records: self.records.clone(),
                next_cursor: None,
            })
        }
        fn surface(&self, _query: SurfaceQuery) -> Result<SearchPage, ApplicationError> {
            unreachable!()
        }
        fn show(&self, record_id: &str) -> Result<SearchRecordDetail, ApplicationError> {
            let record = self
                .records
                .iter()
                .find(|record| record.record_id == record_id)
                .cloned()
                .ok_or_else(|| ApplicationError::NotFound(record_id.to_owned()))?;
            Ok(SearchRecordDetail {
                record,
                revisions: Vec::new(),
                assets: Vec::new(),
                derivatives: Vec::new(),
                relations: Vec::new(),
                score_history: Vec::new(),
            })
        }
        fn traverse(&self, _record_id: &str) -> Result<Vec<SearchRecordDetail>, ApplicationError> {
            unreachable!()
        }
        fn status(&self) -> Result<ProjectionStatus, ApplicationError> {
            Ok(ProjectionStatus {
                state: "ready".to_owned(),
                schema_version: 1,
                built_at: Some(timestamp()),
                raw_items: 2,
                semantic_entries: 1,
                relations: 0,
                source_fingerprint: Some("f".repeat(64)),
            })
        }
    }

    #[test]
    fn definition_explicitly_controls_unreviewed_and_manual_exclusion() {
        let projection = FixtureProjection {
            records: vec![
                record("item:item_01J00000000000000000000000", "external", None),
                record(
                    "semantic:semantic_01J00000000000000000000000",
                    "machine",
                    Some("unreviewed"),
                ),
                record("item:item_01J00000000000000000000001", "external", None),
            ],
        };
        let mut definition = definition(false);
        definition.definition.manual_exclude =
            vec!["item:item_01J00000000000000000000001".to_owned()];
        let excluded = resolve_definition(&projection, definition.clone(), timestamp()).unwrap();
        assert_eq!(excluded.members.len(), 1);
        assert_eq!(excluded.exclusions.len(), 2);
        assert!(excluded.exclusions.iter().any(|entry| {
            entry.record_id.starts_with("semantic:")
                && entry.reason == "unreviewed_excluded_by_definition"
        }));
        definition.definition.include_unreviewed = true;
        let included = resolve_definition(&projection, definition, timestamp()).unwrap();
        assert_eq!(included.members.len(), 2);
        assert!(included.members.iter().any(|member| {
            member.record.review_state.as_deref() == Some("unreviewed")
                && !member.record.judgment.human_judgment
                && !member.record.judgment.confirmed_fact
        }));
    }

    fn definition(include_unreviewed: bool) -> SublibraryDefinition {
        SublibraryDefinition {
            schema_version: SUBLIBRARY_SCHEMA_VERSION.to_owned(),
            id: SublibraryId::parse("sublibrary_01J00000000000000000000000").unwrap(),
            version: 1,
            definition: babata_domain::SublibraryDefinitionInput {
                title: "Fixture".to_owned(),
                purpose: "Exercise explicit downstream suggestion policy".to_owned(),
                selection: babata_domain::QueryFilter {
                    limit: 20,
                    ..babata_domain::QueryFilter::default()
                },
                manual_include: Vec::new(),
                manual_exclude: Vec::new(),
                organisation_rules: vec![SublibraryOrganisationRule::Title],
                include_unreviewed,
            },
            author: "fixture-user".to_owned(),
            created_at: timestamp(),
            authority: None,
        }
    }

    fn record(
        record_id: &str,
        origin_kind: &str,
        review_state: Option<&str>,
    ) -> babata_domain::RecordSummary {
        let semantic = record_id.strip_prefix("semantic:").map(str::to_owned);
        babata_domain::RecordSummary {
            record_id: record_id.to_owned(),
            record_kind: if semantic.is_some() {
                SearchRecordKind::SemanticEntry
            } else {
                SearchRecordKind::RawItem
            },
            item_id: None,
            revision_id: None,
            semantic_id: semantic,
            source_id: SourceId::parse("source_01J00000000000000000000000").unwrap(),
            source_locator: None,
            source_native_id: None,
            title: record_id.to_owned(),
            excerpt: Some("fixture".to_owned()),
            source_kind: SourceKind::External,
            provider: "fixture".to_owned(),
            content_type: ContentType::Text,
            semantic_kind: review_state.map(|_| KnowledgeKind::Knowledge),
            realm: review_state.map(|_| KnowledgeRealm::KnowledgeAndCases),
            state: "ready".to_owned(),
            processing_state: "ready".to_owned(),
            origin_kind: origin_kind.to_owned(),
            review_state: review_state.map(str::to_owned),
            access_state: "accessible".to_owned(),
            judgment: JudgmentStatus {
                human_judgment: false,
                confirmed_fact: false,
            },
            event_at: timestamp(),
            markers: Vec::new(),
            limitations: Vec::new(),
            people: Vec::new(),
            map_nodes: Vec::new(),
            tags: Vec::new(),
            score: None,
            reasons: Vec::new(),
        }
    }

    fn timestamp() -> UtcTimestamp {
        UtcTimestamp::parse("2026-07-23T00:00:00Z").unwrap()
    }
}
