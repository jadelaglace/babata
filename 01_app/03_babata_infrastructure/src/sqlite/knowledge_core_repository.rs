use std::collections::{BTreeSet, HashMap};

use babata_application::ports::KnowledgeCoreRepositoryPort;
use babata_application::{
    ApplicationError, CreateScoreProfileCommand, DenseExpressionDetail, FirstPartySemanticOutcome,
    IngestSemanticCandidateCommand, MapNodeDetail, ModelSuggestionDetail,
    RecordRelevanceScoreCommand, RecordSuggestionReviewCommand, RegisterFirstPartySemanticCommand,
    RelevanceScoreDetail, SemanticCoreSnapshot, SemanticEntryDetail, SemanticIngestOutcome,
    SemanticRelationDetail, SuggestionReviewDetail,
};
use babata_domain::{
    DenseExpressionId, DenseExpressionKind, DerivativeId, ItemId, KnowledgeKind, KnowledgeRealm,
    MapNodeId, MapNodeLevel, RevisionId, ScoreId, ScoreProfile, SemanticId, SemanticRelationId,
    Sha256, SuggestionDecisionKind, SuggestionId, SuggestionReviewId, TagId, UtcTimestamp,
};
use rusqlite::{OptionalExtension, Transaction, params};

use super::SqliteRawRepository;

const MAP_VERSION_ID: &str = "map_version_p6_baseline";

impl KnowledgeCoreRepositoryPort for SqliteRawRepository {
    #[allow(clippy::too_many_lines)]
    fn ingest_machine_candidate(
        &self,
        command: &IngestSemanticCandidateCommand,
    ) -> Result<SemanticIngestOutcome, ApplicationError> {
        command.package.validate().map_err(ApplicationError::from)?;
        let mut connection = self.lock()?;
        let transaction = connection.transaction().map_err(storage)?;
        validate_ready_source(
            &transaction,
            &command.package.source_item_id,
            &command.package.source_revision_id,
        )?;
        let profile = latest_profile(&transaction)?;
        let suggestion_id = SuggestionId::new();
        transaction
            .execute(
                "INSERT INTO model_suggestions
                 (suggestion_id, suggestion_kind, source_item_id, source_revision_id,
                  source_derivative_id, source_output_sha256, provider, model, model_version,
                  prompt_version, generated_at, evidence_derivatives_json, limitations_json,
                  created_at)
                 VALUES (?1, 'semantic_package', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11,
                         ?12, ?10)",
                params![
                    suggestion_id.to_string(),
                    command.package.source_item_id.to_string(),
                    command.package.source_revision_id.to_string(),
                    command.source_derivative_id.to_string(),
                    command.source_output_sha256.as_str(),
                    command.package.provider,
                    command.package.model,
                    command.package.model_version,
                    command.package.prompt_version,
                    command.package.generated_at.as_str(),
                    serde_json::to_string(&command.package.evidence_derivatives).map_err(json)?,
                    serde_json::to_string(&command.package.limitations).map_err(json)?,
                ],
            )
            .map_err(storage)?;

        let mut map_refs = load_map_refs(&transaction)?;
        let mut used_map_nodes = BTreeSet::new();
        for node in &command.package.map_nodes {
            let existing = transaction
                .query_row(
                    "SELECT map_node_id FROM knowledge_map_nodes
                     WHERE map_version_id = ?1 AND node_level = ?2 AND name = ?3",
                    params![MAP_VERSION_ID, map_level(node.level), node.name.trim()],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(storage)?;
            let node_id = existing.unwrap_or_else(|| MapNodeId::new().to_string());
            if !map_refs.values().any(|known| known == &node_id) {
                transaction
                    .execute(
                        "INSERT INTO knowledge_map_nodes
                         (map_node_id, map_version_id, node_level, canonical_key, name,
                          provenance_kind, suggestion_id, created_at)
                         VALUES (?1, ?2, ?3, ?4, ?5, 'machine', ?6, ?7)",
                        params![
                            node_id,
                            MAP_VERSION_ID,
                            map_level(node.level),
                            format!("suggestion:{}:{}", suggestion_id, node.local_ref),
                            node.name.trim(),
                            suggestion_id.to_string(),
                            command.package.generated_at.as_str(),
                        ],
                    )
                    .map_err(storage)?;
            }
            map_refs.insert(node.local_ref.clone(), node_id.clone());
            used_map_nodes.insert(node_id.clone());
            for parent_ref in &node.parent_refs {
                let parent_id = map_refs.get(parent_ref).ok_or_else(|| {
                    ApplicationError::Integrity(format!(
                        "map parent {parent_ref} was not resolved before {}",
                        node.local_ref
                    ))
                })?;
                transaction
                    .execute(
                        "INSERT OR IGNORE INTO knowledge_map_edges
                         (map_version_id, parent_node_id, child_node_id, provenance_kind,
                          suggestion_id, created_at)
                         VALUES (?1, ?2, ?3, 'machine', ?4, ?5)",
                        params![
                            MAP_VERSION_ID,
                            parent_id,
                            node_id,
                            suggestion_id.to_string(),
                            command.package.generated_at.as_str(),
                        ],
                    )
                    .map_err(storage)?;
            }
        }

        let mut semantic_refs = HashMap::new();
        for entry in &command.package.entries {
            let semantic_id = SemanticId::new();
            semantic_refs.insert(entry.local_ref.clone(), semantic_id.to_string());
            transaction
                .execute(
                    "INSERT INTO semantic_entries
                     (semantic_id, semantic_kind, realm, origin_kind, author, title, payload_json,
                      source_item_id, source_revision_id, first_party_item_id,
                      first_party_revision_id, suggestion_id, created_at)
                     VALUES (?1, ?2, ?3, 'machine', ?4, ?5, ?6, ?7, ?8, NULL, NULL, ?9, ?10)",
                    params![
                        semantic_id.to_string(),
                        knowledge_kind(entry.payload.knowledge_kind()),
                        knowledge_realm(entry.payload.knowledge_kind().realm()),
                        command.package.model,
                        entry.title.trim(),
                        serde_json::to_string(&entry.payload).map_err(json)?,
                        command.package.source_item_id.to_string(),
                        command.package.source_revision_id.to_string(),
                        suggestion_id.to_string(),
                        command.package.generated_at.as_str(),
                    ],
                )
                .map_err(storage)?;
            for node_ref in &entry.map_node_refs {
                let map_node_id = map_refs.get(node_ref).ok_or_else(|| {
                    ApplicationError::Integrity(format!("unknown map node ref {node_ref}"))
                })?;
                used_map_nodes.insert(map_node_id.clone());
                transaction
                    .execute(
                        "INSERT INTO semantic_map_assignments
                         (semantic_id, map_node_id, provenance_kind, suggestion_id, created_at)
                         VALUES (?1, ?2, 'machine', ?3, ?4)",
                        params![
                            semantic_id.to_string(),
                            map_node_id,
                            suggestion_id.to_string(),
                            command.package.generated_at.as_str(),
                        ],
                    )
                    .map_err(storage)?;
            }
            for tag in &entry.tags {
                let canonical = tag.trim().to_lowercase();
                let tag_id = transaction
                    .query_row(
                        "SELECT tag_id FROM semantic_tags WHERE canonical_name = ?1",
                        params![canonical],
                        |row| row.get::<_, String>(0),
                    )
                    .optional()
                    .map_err(storage)?
                    .unwrap_or_else(|| TagId::new().to_string());
                transaction
                    .execute(
                        "INSERT OR IGNORE INTO semantic_tags
                         (tag_id, canonical_name, display_name, created_at) VALUES (?1, ?2, ?3, ?4)",
                        params![
                            tag_id,
                            canonical,
                            tag.trim(),
                            command.package.generated_at.as_str(),
                        ],
                    )
                    .map_err(storage)?;
                transaction
                    .execute(
                        "INSERT INTO semantic_tag_assignments
                         (semantic_id, tag_id, provenance_kind, suggestion_id, created_at)
                         VALUES (?1, ?2, 'machine', ?3, ?4)",
                        params![
                            semantic_id.to_string(),
                            tag_id,
                            suggestion_id.to_string(),
                            command.package.generated_at.as_str(),
                        ],
                    )
                    .map_err(storage)?;
            }
            for expression in &entry.dense_expressions {
                transaction
                    .execute(
                        "INSERT INTO dense_expressions
                         (expression_id, semantic_id, expression_kind, content_text,
                          provenance_kind, suggestion_id, created_at)
                         VALUES (?1, ?2, ?3, ?4, 'machine', ?5, ?6)",
                        params![
                            DenseExpressionId::new().to_string(),
                            semantic_id.to_string(),
                            expression_kind(expression.kind),
                            expression.content,
                            suggestion_id.to_string(),
                            command.package.generated_at.as_str(),
                        ],
                    )
                    .map_err(storage)?;
            }
            transaction
                .execute(
                    "INSERT INTO relevance_scores
                     (score_id, target_kind, target_id, profile_id, interest, strategy,
                      consensus, weighted_score, rationale, provenance_kind, author,
                      suggestion_id, created_at)
                     VALUES (?1, 'semantic', ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'machine', ?9, ?10,
                             ?11)",
                    params![
                        ScoreId::new().to_string(),
                        semantic_id.to_string(),
                        profile.profile_id,
                        entry.relevance.interest,
                        entry.relevance.strategy,
                        entry.relevance.consensus,
                        entry.relevance.weighted_score(&profile),
                        entry.relevance.rationale,
                        command.package.model,
                        suggestion_id.to_string(),
                        command.package.generated_at.as_str(),
                    ],
                )
                .map_err(storage)?;
        }
        for relation in &command.package.relations {
            transaction
                .execute(
                    "INSERT INTO semantic_relations
                     (semantic_relation_id, from_semantic_id, relation_kind, to_semantic_id,
                      evidence, provenance_kind, suggestion_id, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, 'machine', ?6, ?7)",
                    params![
                        SemanticRelationId::new().to_string(),
                        semantic_refs[&relation.from_ref],
                        relation.kind.trim(),
                        semantic_refs[&relation.to_ref],
                        relation.evidence.trim(),
                        suggestion_id.to_string(),
                        command.package.generated_at.as_str(),
                    ],
                )
                .map_err(storage)?;
        }
        transaction.commit().map_err(storage)?;
        Ok(SemanticIngestOutcome {
            suggestion_id: suggestion_id.to_string(),
            semantic_ids: semantic_refs.into_values().collect(),
            map_node_ids: used_map_nodes.into_iter().collect(),
            profile_id: profile.profile_id,
            review_state: "unreviewed".to_owned(),
        })
    }

    fn load_semantic_snapshot(
        &self,
        suggestion_id: &str,
    ) -> Result<SemanticCoreSnapshot, ApplicationError> {
        let connection = self.lock()?;
        let suggestion = load_suggestion(&connection, suggestion_id)?;
        let mut entries = Vec::new();
        let mut statement = connection
            .prepare(
                "SELECT semantic_id FROM semantic_entries WHERE suggestion_id = ?1
                 ORDER BY created_at, semantic_id",
            )
            .map_err(storage)?;
        let rows = statement
            .query_map(params![suggestion_id], |row| row.get::<_, String>(0))
            .map_err(storage)?;
        for row in rows {
            entries.push(load_entry(&connection, &row.map_err(storage)?)?);
        }
        drop(statement);
        Ok(SemanticCoreSnapshot {
            suggestion,
            entries,
            relations: load_relations(&connection, suggestion_id)?,
            reviews: load_reviews(&connection, suggestion_id)?,
        })
    }

    fn record_suggestion_review(
        &self,
        command: &RecordSuggestionReviewCommand,
    ) -> Result<(), ApplicationError> {
        if command.reviewer.trim().is_empty() {
            return Err(ApplicationError::Integrity(
                "reviewer must not be blank".to_owned(),
            ));
        }
        match command.decision {
            SuggestionDecisionKind::Modify => {
                let (item, revision) = command
                    .first_party_item_id
                    .as_ref()
                    .zip(command.first_party_revision_id.as_ref())
                    .ok_or_else(|| {
                        ApplicationError::Integrity(
                            "modified review requires new first-party C0 content".to_owned(),
                        )
                    })?;
                let connection = self.lock()?;
                validate_ready_first_party(&connection, item, revision)?;
                insert_review(&connection, command)
            }
            SuggestionDecisionKind::Reject => {
                if command
                    .reason
                    .as_deref()
                    .is_none_or(|reason| reason.trim().is_empty())
                {
                    return Err(ApplicationError::Integrity(
                        "rejected review requires a reason".to_owned(),
                    ));
                }
                if command.first_party_item_id.is_some()
                    || command.first_party_revision_id.is_some()
                {
                    return Err(ApplicationError::Integrity(
                        "rejected review must not claim first-party content".to_owned(),
                    ));
                }
                let connection = self.lock()?;
                insert_review(&connection, command)
            }
            SuggestionDecisionKind::Accept => {
                if command.first_party_item_id.is_some()
                    || command.first_party_revision_id.is_some()
                {
                    return Err(ApplicationError::Integrity(
                        "accepted review does not copy machine content into first-party C0"
                            .to_owned(),
                    ));
                }
                let connection = self.lock()?;
                insert_review(&connection, command)
            }
        }
    }

    fn create_score_profile(
        &self,
        command: &CreateScoreProfileCommand,
    ) -> Result<(), ApplicationError> {
        command.profile.validate().map_err(ApplicationError::from)?;
        if command.author.trim().is_empty()
            || !matches!(command.author_kind.as_str(), "machine" | "first_party")
        {
            return Err(ApplicationError::Integrity(
                "profile author identity is invalid".to_owned(),
            ));
        }
        let connection = self.lock()?;
        let next_ordinal = connection
            .query_row(
                "SELECT COALESCE(MAX(ordinal), 0) + 1 FROM score_profiles",
                [],
                |row| row.get::<_, u32>(0),
            )
            .map_err(storage)?;
        if command.profile.ordinal != next_ordinal {
            return Err(ApplicationError::Conflict(format!(
                "score profile ordinal must be {next_ordinal}"
            )));
        }
        connection
            .execute(
                "INSERT INTO score_profiles
                 (profile_id, ordinal, interest_weight, strategy_weight, consensus_weight,
                  rationale, author_kind, author, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    command.profile.profile_id,
                    command.profile.ordinal,
                    command.profile.interest_weight,
                    command.profile.strategy_weight,
                    command.profile.consensus_weight,
                    command.profile.rationale,
                    command.author_kind,
                    command.author,
                    command.profile.created_at.as_str(),
                ],
            )
            .map_err(storage)?;
        Ok(())
    }

    fn list_score_profiles(&self) -> Result<Vec<ScoreProfile>, ApplicationError> {
        let connection = self.lock()?;
        let mut statement = connection
            .prepare(
                "SELECT profile_id, ordinal, interest_weight, strategy_weight, consensus_weight,
                        rationale, created_at FROM score_profiles ORDER BY ordinal",
            )
            .map_err(storage)?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, u32>(1)?,
                    row.get::<_, u8>(2)?,
                    row.get::<_, u8>(3)?,
                    row.get::<_, u8>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                ))
            })
            .map_err(storage)?;
        rows.map(|row| {
            let (
                profile_id,
                ordinal,
                interest_weight,
                strategy_weight,
                consensus_weight,
                rationale,
                created_at,
            ) = row.map_err(storage)?;
            Ok(ScoreProfile {
                profile_id,
                ordinal,
                interest_weight,
                strategy_weight,
                consensus_weight,
                rationale,
                created_at: UtcTimestamp::parse(created_at).map_err(ApplicationError::from)?,
            })
        })
        .collect()
    }

    #[allow(clippy::too_many_lines)]
    fn register_first_party_semantic(
        &self,
        command: &RegisterFirstPartySemanticCommand,
    ) -> Result<FirstPartySemanticOutcome, ApplicationError> {
        command
            .definition
            .validate()
            .map_err(ApplicationError::from)?;
        if command.author.trim().is_empty() {
            return Err(ApplicationError::Integrity(
                "first-party semantic author must not be blank".to_owned(),
            ));
        }
        let mut connection = self.lock()?;
        validate_ready_first_party(&connection, &command.item_id, &command.revision_id)?;
        let raw_text = connection
            .query_row(
                "SELECT raw_text FROM revisions WHERE revision_id = ?1",
                params![command.revision_id.to_string()],
                |row| row.get::<_, Option<String>>(0),
            )
            .map_err(storage)?
            .ok_or_else(|| {
                ApplicationError::Integrity(
                    "first-party semantic revision must contain authored text".to_owned(),
                )
            })?;
        if !matches!(
            command.definition.payload,
            babata_domain::SemanticPayload::Log { .. }
                | babata_domain::SemanticPayload::Insight { .. }
        ) {
            return Err(ApplicationError::Conflict(
                "first-party semantic registration currently accepts Log or Insight; other authored work stays C0 until its structured contract is proven"
                    .to_owned(),
            ));
        }
        match &command.definition.payload {
            babata_domain::SemanticPayload::Log { body, .. }
            | babata_domain::SemanticPayload::Insight { body, .. }
                if body != &raw_text =>
            {
                return Err(ApplicationError::Integrity(
                    "Log/Insight body must be the exact first-party C0 text".to_owned(),
                ));
            }
            _ => {}
        }
        let transaction = connection.transaction().map_err(storage)?;
        let profile = latest_profile(&transaction)?;
        let semantic_id = SemanticId::new();
        let semantic_kind_value = command.definition.payload.knowledge_kind();
        transaction
            .execute(
                "INSERT INTO semantic_entries
                 (semantic_id, semantic_kind, realm, origin_kind, author, title, payload_json,
                  source_item_id, source_revision_id, first_party_item_id,
                  first_party_revision_id, suggestion_id, created_at)
                 VALUES (?1, ?2, ?3, 'first_party', ?4, ?5, ?6, NULL, NULL, ?7, ?8, NULL, ?9)",
                params![
                    semantic_id.to_string(),
                    knowledge_kind(semantic_kind_value),
                    knowledge_realm(semantic_kind_value.realm()),
                    command.author.trim(),
                    command.definition.title.trim(),
                    serde_json::to_string(&command.definition.payload).map_err(json)?,
                    command.item_id.to_string(),
                    command.revision_id.to_string(),
                    command.created_at.as_str(),
                ],
            )
            .map_err(storage)?;
        for node_ref in &command.definition.map_node_refs {
            let map_node_id = transaction
                .query_row(
                    "SELECT map_node_id FROM knowledge_map_nodes
                     WHERE map_version_id = ?1 AND (canonical_key = ?2 OR map_node_id = ?2)",
                    params![MAP_VERSION_ID, node_ref],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(storage)?
                .ok_or_else(|| ApplicationError::NotFound(node_ref.clone()))?;
            transaction
                .execute(
                    "INSERT INTO semantic_map_assignments
                     (semantic_id, map_node_id, provenance_kind, suggestion_id, created_at)
                     VALUES (?1, ?2, 'first_party', NULL, ?3)",
                    params![
                        semantic_id.to_string(),
                        map_node_id,
                        command.created_at.as_str()
                    ],
                )
                .map_err(storage)?;
        }
        for tag in &command.definition.tags {
            let canonical = tag.trim().to_lowercase();
            let tag_id = transaction
                .query_row(
                    "SELECT tag_id FROM semantic_tags WHERE canonical_name = ?1",
                    params![canonical],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .map_err(storage)?
                .unwrap_or_else(|| TagId::new().to_string());
            transaction
                .execute(
                    "INSERT OR IGNORE INTO semantic_tags
                     (tag_id, canonical_name, display_name, created_at) VALUES (?1, ?2, ?3, ?4)",
                    params![tag_id, canonical, tag.trim(), command.created_at.as_str()],
                )
                .map_err(storage)?;
            transaction
                .execute(
                    "INSERT INTO semantic_tag_assignments
                     (semantic_id, tag_id, provenance_kind, suggestion_id, created_at)
                     VALUES (?1, ?2, 'first_party', NULL, ?3)",
                    params![semantic_id.to_string(), tag_id, command.created_at.as_str()],
                )
                .map_err(storage)?;
        }
        for expression in &command.definition.dense_expressions {
            transaction
                .execute(
                    "INSERT INTO dense_expressions
                     (expression_id, semantic_id, expression_kind, content_text,
                      provenance_kind, suggestion_id, created_at)
                     VALUES (?1, ?2, ?3, ?4, 'first_party', NULL, ?5)",
                    params![
                        DenseExpressionId::new().to_string(),
                        semantic_id.to_string(),
                        expression_kind(expression.kind),
                        expression.content,
                        command.created_at.as_str(),
                    ],
                )
                .map_err(storage)?;
        }
        transaction
            .execute(
                "INSERT INTO relevance_scores
                 (score_id, target_kind, target_id, profile_id, interest, strategy,
                  consensus, weighted_score, rationale, provenance_kind, author,
                  suggestion_id, created_at)
                 VALUES (?1, 'semantic', ?2, ?3, ?4, ?5, ?6, ?7, ?8, 'first_party', ?9,
                         NULL, ?10)",
                params![
                    ScoreId::new().to_string(),
                    semantic_id.to_string(),
                    profile.profile_id,
                    command.definition.relevance.interest,
                    command.definition.relevance.strategy,
                    command.definition.relevance.consensus,
                    command.definition.relevance.weighted_score(&profile),
                    command.definition.relevance.rationale,
                    command.author.trim(),
                    command.created_at.as_str(),
                ],
            )
            .map_err(storage)?;
        for relation in &command.definition.relations {
            let target_exists = transaction
                .query_row(
                    "SELECT 1 FROM semantic_entries WHERE semantic_id = ?1",
                    params![relation.to_semantic_id],
                    |_| Ok(()),
                )
                .is_ok();
            if !target_exists || relation.to_semantic_id == semantic_id.to_string() {
                return Err(ApplicationError::NotFound(relation.to_semantic_id.clone()));
            }
            transaction
                .execute(
                    "INSERT INTO semantic_relations
                     (semantic_relation_id, from_semantic_id, relation_kind, to_semantic_id,
                      evidence, provenance_kind, suggestion_id, created_at)
                     VALUES (?1, ?2, ?3, ?4, ?5, 'first_party', NULL, ?6)",
                    params![
                        SemanticRelationId::new().to_string(),
                        semantic_id.to_string(),
                        relation.kind.trim(),
                        relation.to_semantic_id,
                        relation.evidence.trim(),
                        command.created_at.as_str(),
                    ],
                )
                .map_err(storage)?;
        }
        transaction.commit().map_err(storage)?;
        Ok(FirstPartySemanticOutcome {
            semantic_id: semantic_id.to_string(),
            kind: semantic_kind_value,
            realm: semantic_kind_value.realm(),
            origin_kind: "first_party".to_owned(),
            first_party_item_id: command.item_id.clone(),
            first_party_revision_id: command.revision_id.clone(),
        })
    }

    fn load_semantic_entry(
        &self,
        semantic_id: &str,
    ) -> Result<SemanticEntryDetail, ApplicationError> {
        let connection = self.lock()?;
        load_entry(&connection, semantic_id)
    }

    fn record_relevance_score(
        &self,
        command: &RecordRelevanceScoreCommand,
    ) -> Result<RelevanceScoreDetail, ApplicationError> {
        command
            .components
            .validate()
            .map_err(ApplicationError::from)?;
        if command.author.trim().is_empty()
            || !matches!(command.author_kind.as_str(), "machine" | "first_party")
        {
            return Err(ApplicationError::Integrity(
                "score author identity is invalid".to_owned(),
            ));
        }
        let connection = self.lock()?;
        if connection
            .query_row(
                "SELECT 1 FROM semantic_entries WHERE semantic_id = ?1",
                params![command.semantic_id],
                |_| Ok(()),
            )
            .is_err()
        {
            return Err(ApplicationError::NotFound(command.semantic_id.clone()));
        }
        let profile = latest_profile(&connection)?;
        let score_id = ScoreId::new().to_string();
        let weighted_score = command.components.weighted_score(&profile);
        connection
            .execute(
                "INSERT INTO relevance_scores
                 (score_id, target_kind, target_id, profile_id, interest, strategy,
                  consensus, weighted_score, rationale, provenance_kind, author,
                  suggestion_id, created_at)
                 VALUES (?1, 'semantic', ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, NULL, ?11)",
                params![
                    score_id,
                    command.semantic_id,
                    profile.profile_id,
                    command.components.interest,
                    command.components.strategy,
                    command.components.consensus,
                    weighted_score,
                    command.components.rationale,
                    command.author_kind,
                    command.author.trim(),
                    command.created_at.as_str(),
                ],
            )
            .map_err(storage)?;
        Ok(RelevanceScoreDetail {
            score_id,
            profile_id: profile.profile_id,
            interest: command.components.interest,
            strategy: command.components.strategy,
            consensus: command.components.consensus,
            weighted_score,
            rationale: command.components.rationale.clone(),
        })
    }
}

fn validate_ready_source(
    transaction: &Transaction<'_>,
    item_id: &ItemId,
    revision_id: &RevisionId,
) -> Result<(), ApplicationError> {
    let ready = transaction
        .query_row(
            "SELECT 1 FROM revisions WHERE revision_id = ?1 AND item_id = ?2 AND state = 'ready'",
            params![revision_id.to_string(), item_id.to_string()],
            |_| Ok(()),
        )
        .is_ok();
    if ready {
        Ok(())
    } else {
        Err(ApplicationError::Integrity(
            "semantic package source is not the requested ready C0 revision".to_owned(),
        ))
    }
}

fn validate_ready_first_party(
    connection: &rusqlite::Connection,
    item_id: &ItemId,
    revision_id: &RevisionId,
) -> Result<(), ApplicationError> {
    let valid = connection
        .query_row(
            "SELECT 1 FROM revisions revision
             JOIN items item ON item.item_id = revision.item_id
             JOIN sources source ON source.source_id = item.source_id
             WHERE revision.revision_id = ?1 AND item.item_id = ?2
               AND revision.state = 'ready' AND source.source_kind = 'first_party'",
            params![revision_id.to_string(), item_id.to_string()],
            |_| Ok(()),
        )
        .is_ok();
    if valid {
        Ok(())
    } else {
        Err(ApplicationError::Integrity(
            "review content is not a ready first-party C0 revision".to_owned(),
        ))
    }
}

fn latest_profile(connection: &rusqlite::Connection) -> Result<ScoreProfile, ApplicationError> {
    let (
        profile_id,
        ordinal,
        interest_weight,
        strategy_weight,
        consensus_weight,
        rationale,
        created_at,
    ) = connection
        .query_row(
            "SELECT profile_id, ordinal, interest_weight, strategy_weight, consensus_weight,
                    rationale, created_at FROM score_profiles ORDER BY ordinal DESC LIMIT 1",
            [],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, u32>(1)?,
                    row.get::<_, u8>(2)?,
                    row.get::<_, u8>(3)?,
                    row.get::<_, u8>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                ))
            },
        )
        .map_err(storage)?;
    Ok(ScoreProfile {
        profile_id,
        ordinal,
        interest_weight,
        strategy_weight,
        consensus_weight,
        rationale,
        created_at: UtcTimestamp::parse(created_at).map_err(ApplicationError::from)?,
    })
}

fn load_map_refs(
    connection: &rusqlite::Connection,
) -> Result<HashMap<String, String>, ApplicationError> {
    let mut statement = connection
        .prepare(
            "SELECT canonical_key, map_node_id FROM knowledge_map_nodes
             WHERE map_version_id = ?1",
        )
        .map_err(storage)?;
    let rows = statement
        .query_map(params![MAP_VERSION_ID], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(storage)?;
    rows.map(|row| row.map_err(storage)).collect()
}

fn load_suggestion(
    connection: &rusqlite::Connection,
    suggestion_id: &str,
) -> Result<ModelSuggestionDetail, ApplicationError> {
    connection
        .query_row(
            "SELECT suggestion.source_item_id, suggestion.source_revision_id,
                    suggestion.source_derivative_id, suggestion.source_output_sha256,
                    suggestion.provider, suggestion.model, suggestion.model_version,
                    suggestion.prompt_version, suggestion.generated_at,
                    suggestion.evidence_derivatives_json, suggestion.limitations_json,
                    COALESCE((SELECT review.decision FROM suggestion_reviews review
                              WHERE review.suggestion_id = suggestion.suggestion_id
                              ORDER BY review.created_at DESC, review.review_id DESC LIMIT 1),
                             'unreviewed')
             FROM model_suggestions suggestion WHERE suggestion.suggestion_id = ?1",
            params![suggestion_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                    row.get::<_, String>(9)?,
                    row.get::<_, String>(10)?,
                    row.get::<_, String>(11)?,
                ))
            },
        )
        .optional()
        .map_err(storage)?
        .ok_or_else(|| ApplicationError::NotFound(suggestion_id.to_owned()))
        .and_then(
            |(
                item,
                revision,
                derivative,
                hash,
                provider,
                model,
                model_version,
                prompt_version,
                generated_at,
                evidence_derivatives,
                limitations,
                review_state,
            )| {
                Ok(ModelSuggestionDetail {
                    suggestion_id: suggestion_id.to_owned(),
                    source_item_id: ItemId::parse(item).map_err(ApplicationError::from)?,
                    source_revision_id: RevisionId::parse(revision)
                        .map_err(ApplicationError::from)?,
                    source_derivative_id: DerivativeId::parse(derivative)
                        .map_err(ApplicationError::from)?,
                    source_output_sha256: Sha256::parse(hash).map_err(ApplicationError::from)?,
                    provider,
                    model,
                    model_version,
                    prompt_version,
                    generated_at: UtcTimestamp::parse(generated_at)
                        .map_err(ApplicationError::from)?,
                    evidence_derivatives: serde_json::from_str(&evidence_derivatives)
                        .map_err(json)?,
                    limitations: serde_json::from_str(&limitations).map_err(json)?,
                    review_state,
                })
            },
        )
}

fn load_entry(
    connection: &rusqlite::Connection,
    semantic_id: &str,
) -> Result<SemanticEntryDetail, ApplicationError> {
    let (kind, realm, origin_kind, author, title, payload_json) = connection
        .query_row(
            "SELECT semantic_kind, realm, origin_kind, author, title, payload_json
             FROM semantic_entries WHERE semantic_id = ?1",
            params![semantic_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )
        .optional()
        .map_err(storage)?
        .ok_or_else(|| ApplicationError::NotFound(semantic_id.to_owned()))?;
    Ok(SemanticEntryDetail {
        semantic_id: semantic_id.to_owned(),
        kind: parse_knowledge_kind(&kind)?,
        realm: parse_knowledge_realm(&realm)?,
        origin_kind,
        author,
        title,
        payload: serde_json::from_str(&payload_json).map_err(json)?,
        map_nodes: load_entry_map_nodes(connection, semantic_id)?,
        tags: load_entry_tags(connection, semantic_id)?,
        dense_expressions: load_entry_expressions(connection, semantic_id)?,
        scores: load_entry_scores(connection, semantic_id)?,
        outgoing_relations: load_entry_relations(connection, semantic_id, true)?,
        incoming_relations: load_entry_relations(connection, semantic_id, false)?,
    })
}

fn load_entry_map_nodes(
    connection: &rusqlite::Connection,
    semantic_id: &str,
) -> Result<Vec<MapNodeDetail>, ApplicationError> {
    let mut statement = connection
        .prepare(
            "SELECT node.map_node_id, node.node_level, node.canonical_key, node.name
         FROM semantic_map_assignments assignment JOIN knowledge_map_nodes node
           ON node.map_node_id = assignment.map_node_id
         WHERE assignment.semantic_id = ?1 ORDER BY node.node_level, node.name",
        )
        .map_err(storage)?;
    let rows = statement
        .query_map(params![semantic_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(storage)?;
    rows.map(|row| {
        let (map_node_id, level, canonical_key, name) = row.map_err(storage)?;
        Ok(MapNodeDetail {
            parent_node_ids: load_map_parents(connection, &map_node_id)?,
            map_node_id,
            level: parse_map_level(&level)?,
            canonical_key,
            name,
        })
    })
    .collect()
}

fn load_map_parents(
    connection: &rusqlite::Connection,
    map_node_id: &str,
) -> Result<Vec<String>, ApplicationError> {
    let mut statement = connection
        .prepare(
            "SELECT parent_node_id FROM knowledge_map_edges
             WHERE child_node_id = ?1 ORDER BY parent_node_id",
        )
        .map_err(storage)?;
    let rows = statement
        .query_map(params![map_node_id], |row| row.get::<_, String>(0))
        .map_err(storage)?;
    rows.map(|row| row.map_err(storage)).collect()
}

fn load_entry_relations(
    connection: &rusqlite::Connection,
    semantic_id: &str,
    outgoing: bool,
) -> Result<Vec<SemanticRelationDetail>, ApplicationError> {
    let sql = if outgoing {
        "SELECT semantic_relation_id, from_semantic_id, relation_kind, to_semantic_id, evidence
         FROM semantic_relations WHERE from_semantic_id = ?1
         ORDER BY created_at, semantic_relation_id"
    } else {
        "SELECT semantic_relation_id, from_semantic_id, relation_kind, to_semantic_id, evidence
         FROM semantic_relations WHERE to_semantic_id = ?1
         ORDER BY created_at, semantic_relation_id"
    };
    let mut statement = connection.prepare(sql).map_err(storage)?;
    let rows = statement
        .query_map(params![semantic_id], |row| {
            Ok(SemanticRelationDetail {
                relation_id: row.get(0)?,
                from_semantic_id: row.get(1)?,
                kind: row.get(2)?,
                to_semantic_id: row.get(3)?,
                evidence: row.get(4)?,
            })
        })
        .map_err(storage)?;
    rows.map(|row| row.map_err(storage)).collect()
}

fn load_entry_tags(
    connection: &rusqlite::Connection,
    semantic_id: &str,
) -> Result<Vec<String>, ApplicationError> {
    let mut statement = connection
        .prepare(
            "SELECT tag.display_name FROM semantic_tag_assignments assignment
         JOIN semantic_tags tag ON tag.tag_id = assignment.tag_id
         WHERE assignment.semantic_id = ?1 ORDER BY tag.canonical_name",
        )
        .map_err(storage)?;
    let rows = statement
        .query_map(params![semantic_id], |row| row.get::<_, String>(0))
        .map_err(storage)?;
    rows.map(|row| row.map_err(storage)).collect()
}

fn load_entry_expressions(
    connection: &rusqlite::Connection,
    semantic_id: &str,
) -> Result<Vec<DenseExpressionDetail>, ApplicationError> {
    let mut statement = connection
        .prepare(
            "SELECT expression_id, expression_kind, content_text FROM dense_expressions
         WHERE semantic_id = ?1 ORDER BY created_at, expression_id",
        )
        .map_err(storage)?;
    let rows = statement
        .query_map(params![semantic_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(storage)?;
    rows.map(|row| {
        let (expression_id, kind, content) = row.map_err(storage)?;
        Ok(DenseExpressionDetail {
            expression_id,
            kind: parse_expression_kind(&kind)?,
            content,
        })
    })
    .collect()
}

fn load_entry_scores(
    connection: &rusqlite::Connection,
    semantic_id: &str,
) -> Result<Vec<RelevanceScoreDetail>, ApplicationError> {
    let mut statement = connection
        .prepare(
            "SELECT score_id, profile_id, interest, strategy, consensus, weighted_score, rationale
         FROM relevance_scores WHERE target_kind = 'semantic' AND target_id = ?1
         ORDER BY created_at, score_id",
        )
        .map_err(storage)?;
    let rows = statement
        .query_map(params![semantic_id], |row| {
            Ok(RelevanceScoreDetail {
                score_id: row.get(0)?,
                profile_id: row.get(1)?,
                interest: row.get(2)?,
                strategy: row.get(3)?,
                consensus: row.get(4)?,
                weighted_score: row.get(5)?,
                rationale: row.get(6)?,
            })
        })
        .map_err(storage)?;
    rows.map(|row| row.map_err(storage)).collect()
}

fn load_relations(
    connection: &rusqlite::Connection,
    suggestion_id: &str,
) -> Result<Vec<SemanticRelationDetail>, ApplicationError> {
    let mut statement = connection.prepare(
        "SELECT semantic_relation_id, from_semantic_id, relation_kind, to_semantic_id, evidence
         FROM semantic_relations WHERE suggestion_id = ?1 ORDER BY created_at, semantic_relation_id"
    ).map_err(storage)?;
    let rows = statement
        .query_map(params![suggestion_id], |row| {
            Ok(SemanticRelationDetail {
                relation_id: row.get(0)?,
                from_semantic_id: row.get(1)?,
                kind: row.get(2)?,
                to_semantic_id: row.get(3)?,
                evidence: row.get(4)?,
            })
        })
        .map_err(storage)?;
    rows.map(|row| row.map_err(storage)).collect()
}

fn load_reviews(
    connection: &rusqlite::Connection,
    suggestion_id: &str,
) -> Result<Vec<SuggestionReviewDetail>, ApplicationError> {
    let mut statement = connection
        .prepare(
            "SELECT review_id, decision, reason, first_party_item_id, first_party_revision_id,
                reviewer, created_at FROM suggestion_reviews WHERE suggestion_id = ?1
         ORDER BY created_at, review_id",
        )
        .map_err(storage)?;
    let rows = statement
        .query_map(params![suggestion_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
            ))
        })
        .map_err(storage)?;
    rows.map(|row| {
        let (review_id, decision, reason, item, revision, reviewer, created_at) =
            row.map_err(storage)?;
        Ok(SuggestionReviewDetail {
            review_id,
            decision: parse_decision(&decision)?,
            reason,
            first_party_item_id: item
                .map(ItemId::parse)
                .transpose()
                .map_err(ApplicationError::from)?,
            first_party_revision_id: revision
                .map(RevisionId::parse)
                .transpose()
                .map_err(ApplicationError::from)?,
            reviewer,
            created_at: UtcTimestamp::parse(created_at).map_err(ApplicationError::from)?,
        })
    })
    .collect()
}

fn insert_review(
    connection: &rusqlite::Connection,
    command: &RecordSuggestionReviewCommand,
) -> Result<(), ApplicationError> {
    connection
        .execute(
            "INSERT INTO suggestion_reviews
         (review_id, suggestion_id, decision, reason, first_party_item_id,
          first_party_revision_id, reviewer, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                SuggestionReviewId::new().to_string(),
                command.suggestion_id,
                decision(command.decision),
                command.reason,
                command
                    .first_party_item_id
                    .as_ref()
                    .map(ToString::to_string),
                command
                    .first_party_revision_id
                    .as_ref()
                    .map(ToString::to_string),
                command.reviewer.trim(),
                command.created_at.as_str(),
            ],
        )
        .map_err(storage)?;
    Ok(())
}

const fn map_level(value: MapNodeLevel) -> &'static str {
    match value {
        MapNodeLevel::Foundation => "foundation",
        MapNodeLevel::Discipline => "discipline",
        MapNodeLevel::Branch => "branch",
    }
}
const fn knowledge_kind(value: KnowledgeKind) -> &'static str {
    match value {
        KnowledgeKind::MapDirection => "map_direction",
        KnowledgeKind::Knowledge => "knowledge",
        KnowledgeKind::Case => "case",
        KnowledgeKind::Log => "log",
        KnowledgeKind::Insight => "insight",
    }
}
const fn knowledge_realm(value: KnowledgeRealm) -> &'static str {
    match value {
        KnowledgeRealm::KnowledgeMap => "knowledge_map",
        KnowledgeRealm::KnowledgeAndCases => "knowledge_and_cases",
        KnowledgeRealm::CognitiveTrail => "cognitive_trail",
    }
}
const fn expression_kind(value: DenseExpressionKind) -> &'static str {
    match value {
        DenseExpressionKind::MindMap => "mind_map",
        DenseExpressionKind::Mermaid => "mermaid",
        DenseExpressionKind::Model => "model",
        DenseExpressionKind::Formula => "formula",
        DenseExpressionKind::Checklist => "checklist",
        DenseExpressionKind::Process => "process",
        DenseExpressionKind::Outline => "outline",
    }
}
const fn decision(value: SuggestionDecisionKind) -> &'static str {
    match value {
        SuggestionDecisionKind::Accept => "accepted",
        SuggestionDecisionKind::Modify => "modified",
        SuggestionDecisionKind::Reject => "rejected",
    }
}

fn parse_map_level(value: &str) -> Result<MapNodeLevel, ApplicationError> {
    match value {
        "foundation" => Ok(MapNodeLevel::Foundation),
        "discipline" => Ok(MapNodeLevel::Discipline),
        "branch" => Ok(MapNodeLevel::Branch),
        _ => Err(invalid_wire("map node level", value)),
    }
}
fn parse_knowledge_kind(value: &str) -> Result<KnowledgeKind, ApplicationError> {
    match value {
        "map_direction" => Ok(KnowledgeKind::MapDirection),
        "knowledge" => Ok(KnowledgeKind::Knowledge),
        "case" => Ok(KnowledgeKind::Case),
        "log" => Ok(KnowledgeKind::Log),
        "insight" => Ok(KnowledgeKind::Insight),
        _ => Err(invalid_wire("knowledge kind", value)),
    }
}
fn parse_knowledge_realm(value: &str) -> Result<KnowledgeRealm, ApplicationError> {
    match value {
        "knowledge_map" => Ok(KnowledgeRealm::KnowledgeMap),
        "knowledge_and_cases" => Ok(KnowledgeRealm::KnowledgeAndCases),
        "cognitive_trail" => Ok(KnowledgeRealm::CognitiveTrail),
        _ => Err(invalid_wire("knowledge realm", value)),
    }
}
fn parse_expression_kind(value: &str) -> Result<DenseExpressionKind, ApplicationError> {
    match value {
        "mind_map" => Ok(DenseExpressionKind::MindMap),
        "mermaid" => Ok(DenseExpressionKind::Mermaid),
        "model" => Ok(DenseExpressionKind::Model),
        "formula" => Ok(DenseExpressionKind::Formula),
        "checklist" => Ok(DenseExpressionKind::Checklist),
        "process" => Ok(DenseExpressionKind::Process),
        "outline" => Ok(DenseExpressionKind::Outline),
        _ => Err(invalid_wire("expression kind", value)),
    }
}
fn parse_decision(value: &str) -> Result<SuggestionDecisionKind, ApplicationError> {
    match value {
        "accepted" => Ok(SuggestionDecisionKind::Accept),
        "modified" => Ok(SuggestionDecisionKind::Modify),
        "rejected" => Ok(SuggestionDecisionKind::Reject),
        _ => Err(invalid_wire("suggestion decision", value)),
    }
}

fn invalid_wire(field: &str, value: &str) -> ApplicationError {
    ApplicationError::Integrity(format!("invalid stored {field}: {value}"))
}
fn storage(error: rusqlite::Error) -> ApplicationError {
    ApplicationError::Storage(error.to_string())
}
fn json(error: serde_json::Error) -> ApplicationError {
    ApplicationError::Integrity(error.to_string())
}
