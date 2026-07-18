use babata_domain::{
    AssetRole, ContentType, ItemId, Metadata, RawState, RelationKind, RevisionId, RevisionKind,
    SourceId, SourceKind, TextPayload, UtcTimestamp,
};

use crate::{
    AnnotateCommand, ApplicationError, CaptureOutcome, CreateNoteCommand, ReviseCommand,
    ports::{
        AssetStorePort, ClockPort, NewAsset, NewCaptureOperation, NewItem, NewRelation,
        NewRevision, NewSource, RawRepositoryPort,
    },
};

use super::{CaptureService, capture::collection_from_context};

fn first_party_source(now: UtcTimestamp) -> NewSource {
    // Keep authored material under one stable built-in source, not one source per note.
    NewSource {
        id: SourceId::parse("source_01J00000000000000000000000").expect("fixed source ID is valid"),
        kind: SourceKind::FirstParty,
        provider: "babata".to_owned(),
        account_or_workspace: None,
        created_at: now,
    }
}

pub struct WorkspaceService<R, A, C> {
    capture: CaptureService<R, A, C>,
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::*;
    use crate::usecases::capture::tests::{FixedClock, MockAssets, MockRepository};

    #[test]
    fn create_then_revise_preserves_parent_lineage() {
        let repository = MockRepository::default();
        let service = WorkspaceService::new(repository.clone(), MockAssets::default(), FixedClock);
        let original = service
            .create(CreateNoteCommand {
                text: "v1".to_owned(),
                path: None,
                context: None,
                metadata: Metadata::empty(),
            })
            .unwrap();
        let revision = service
            .revise(ReviseCommand {
                parent: original.revision_id.clone(),
                text: "v2".to_owned(),
                path: None,
                note: None,
                metadata: Metadata::empty(),
            })
            .unwrap();
        let state = repository.state.lock().unwrap();
        let child = state
            .revisions
            .iter()
            .find(|entry| entry.id == revision.revision_id)
            .unwrap();
        assert_eq!(child.parent_revision_id, Some(original.revision_id));
        assert_eq!(child.ordinal, 2);
    }
    #[test]
    fn annotation_creates_a_separate_item_relation() {
        let repository = MockRepository::default();
        let service = WorkspaceService::new(repository.clone(), MockAssets::default(), FixedClock);
        let original = service
            .create(CreateNoteCommand {
                text: "source".to_owned(),
                path: None,
                context: None,
                metadata: Metadata::empty(),
            })
            .unwrap();
        let annotation = service
            .annotate_revision(
                original.revision_id.clone(),
                "my note".to_owned(),
                None,
                Metadata::empty(),
            )
            .unwrap();
        let state = repository.state.lock().unwrap();
        assert_ne!(original.item_id, annotation.item_id);
        assert!(
            state
                .relations
                .iter()
                .any(|relation| relation.kind == RelationKind::Annotates
                    && relation.from_revision_id == Some(annotation.revision_id.clone())
                    && relation.to_revision_id.as_ref() == Some(&original.revision_id))
        );
    }

    #[test]
    fn external_revision_cannot_be_revised_as_first_party() {
        let repository = MockRepository::default();
        let capture = CaptureService::new(repository.clone(), MockAssets::default(), FixedClock);
        let external = capture
            .capture_text(crate::CaptureTextCommand {
                provider: "fixture".to_owned(),
                text: "external".to_owned(),
                context: None,
                locator: None,
                native_id: None,
                identity: None,
                metadata: Metadata::empty(),
                source_published_at: None,
            })
            .unwrap();
        let workspace = WorkspaceService::new(repository, MockAssets::default(), FixedClock);
        assert!(matches!(
            workspace.revise(ReviseCommand {
                parent: external.revision_id,
                text: "not an edit".to_owned(),
                path: None,
                note: None,
                metadata: Metadata::empty(),
            }),
            Err(ApplicationError::Conflict(_))
        ));
    }
}

impl<R, A, C> WorkspaceService<R, A, C>
where
    R: RawRepositoryPort,
    A: AssetStorePort,
    C: ClockPort,
{
    pub fn new(repository: R, assets: A, clock: C) -> Self {
        Self {
            capture: CaptureService::new(repository, assets, clock),
        }
    }

    pub fn create(&self, command: CreateNoteCommand) -> Result<CaptureOutcome, ApplicationError> {
        let now = self.capture.clock.now();
        let source = self
            .capture
            .repository
            .find_source(SourceKind::FirstParty, "babata", None)?
            .unwrap_or_else(|| first_party_source(now.clone()));
        let item = NewItem {
            id: ItemId::new(),
            source_id: source.id.clone(),
            source_native_id: None,
            source_locator: None,
            source_identity_key: None,
            content_type: ContentType::Text,
            source_published_at: None,
            first_captured_at: now.clone(),
            metadata: command.metadata,
        };
        let collection = collection_from_context(&source, command.context, now.clone())?;
        self.write_note(
            source,
            collection,
            item,
            None,
            RevisionKind::Authored,
            command.text,
            command.path,
            None,
            Metadata::empty(),
            Vec::new(),
            now,
        )
    }

    pub fn revise(&self, command: ReviseCommand) -> Result<CaptureOutcome, ApplicationError> {
        let parent = self
            .capture
            .repository
            .find_revision(&command.parent)?
            .ok_or_else(|| ApplicationError::NotFound(command.parent.to_string()))?;
        let item = self
            .capture
            .repository
            .find_item(&parent.item_id)?
            .ok_or_else(|| ApplicationError::Integrity("parent item is missing".to_owned()))?;
        let state = self
            .capture
            .repository
            .find_revision_state(&command.parent)?
            .ok_or_else(|| ApplicationError::NotFound(command.parent.to_string()))?;
        if state != RawState::Ready {
            return Err(ApplicationError::Conflict(
                "only a ready first-party revision can be revised".to_owned(),
            ));
        }
        let source = first_party_source(self.capture.clock.now());
        if item.source_id != source.id {
            return Err(ApplicationError::Conflict(
                "external material must be annotated, not revised as first-party".to_owned(),
            ));
        }
        self.write_note(
            source,
            None,
            item,
            Some(parent.id),
            RevisionKind::Edit,
            command.text,
            command.path,
            command.note,
            command.metadata,
            Vec::new(),
            self.capture.clock.now(),
        )
    }

    pub fn annotate(&self, command: AnnotateCommand) -> Result<CaptureOutcome, ApplicationError> {
        let now = self.capture.clock.now();
        self.capture
            .repository
            .find_item(&command.target_item)?
            .ok_or_else(|| ApplicationError::NotFound(command.target_item.to_string()))?;
        let target_detail = self.capture.repository.load_detail(&command.target_item)?;
        let target_revision = match command.target_revision {
            Some(revision_id) => target_detail
                .revisions
                .iter()
                .find(|revision| {
                    revision.revision_id == revision_id && revision.state == RawState::Ready
                })
                .map(|revision| revision.revision_id.clone())
                .ok_or_else(|| {
                    ApplicationError::Conflict(
                        "annotation target revision is not a ready version of the target item"
                            .to_owned(),
                    )
                })?,
            None => target_detail
                .revisions
                .iter()
                .rev()
                .find(|revision| revision.state == RawState::Ready)
                .map(|revision| revision.revision_id.clone())
                .ok_or_else(|| ApplicationError::NotFound(command.target_item.to_string()))?,
        };
        let source = self
            .capture
            .repository
            .find_source(SourceKind::FirstParty, "babata", None)?
            .unwrap_or_else(|| first_party_source(now.clone()));
        let item = NewItem {
            id: ItemId::new(),
            source_id: source.id.clone(),
            source_native_id: None,
            source_locator: None,
            source_identity_key: None,
            content_type: ContentType::Text,
            source_published_at: None,
            first_captured_at: now.clone(),
            metadata: command.metadata,
        };
        let relation = NewRelation {
            kind: RelationKind::Annotates,
            from_item_id: item.id.clone(),
            from_revision_id: None,
            to_item_id: command.target_item,
            to_revision_id: Some(target_revision),
        };
        let collection = collection_from_context(&source, command.context, now.clone())?;
        self.write_note(
            source,
            collection,
            item,
            None,
            RevisionKind::Annotation,
            command.text,
            command.path,
            None,
            Metadata::empty(),
            vec![relation],
            now,
        )
    }

    pub fn annotate_revision(
        &self,
        revision_id: RevisionId,
        text: String,
        path: Option<String>,
        metadata: Metadata,
    ) -> Result<CaptureOutcome, ApplicationError> {
        let target = self
            .capture
            .repository
            .find_revision(&revision_id)?
            .ok_or_else(|| ApplicationError::NotFound(revision_id.to_string()))?;
        if self.capture.repository.find_revision_state(&revision_id)? != Some(RawState::Ready) {
            return Err(ApplicationError::Conflict(
                "only a ready revision can be annotated".to_owned(),
            ));
        }
        self.annotate(AnnotateCommand {
            target_item: target.item_id,
            target_revision: Some(revision_id),
            text,
            path,
            context: None,
            metadata,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn write_note(
        &self,
        source: NewSource,
        collection: Option<crate::ports::NewCollection>,
        item: NewItem,
        parent: Option<RevisionId>,
        kind: RevisionKind,
        text: String,
        path: Option<String>,
        note: Option<String>,
        revision_metadata: Metadata,
        relations: Vec<NewRelation>,
        now: UtcTimestamp,
    ) -> Result<CaptureOutcome, ApplicationError> {
        let payload = TextPayload::new(text)?;
        let operation_id = format!("op_{}", ulid::Ulid::new());
        let staged = match path
            .map(|path| {
                self.capture
                    .assets
                    .stage(&path, AssetRole::Original, &operation_id)
            })
            .transpose()
        {
            Ok(staged) => staged,
            Err(error) => {
                let _ = self.capture.assets.complete_operation(&operation_id);
                return Err(error.with_operation(operation_id));
            }
        };
        let result = (|| {
            let ordinal = if parent.is_some() {
                self.capture.repository.next_ordinal(&item.id)?
            } else {
                1
            };
            let revision = NewRevision {
                id: RevisionId::new(),
                item_id: item.id.clone(),
                parent_revision_id: parent.clone(),
                kind,
                ordinal,
                captured_at: now.clone(),
                authored_at: Some(now.clone()),
                revision_note: note,
                raw_text: Some(payload.as_str().to_owned()),
                text_sha256: Some(payload.hash()),
                metadata: revision_metadata.clone(),
            };
            let operation = NewCaptureOperation {
                operation_id: operation_id.clone(),
                item_id: item.id.clone(),
                revision_id: revision.id.clone(),
                source_native_id: None,
                source_locator: None,
                source_published_at: None,
                metadata: revision_metadata,
                started_at: now,
            };
            let mut all_relations = relations;
            for relation in &mut all_relations {
                if relation.from_item_id == item.id && relation.from_revision_id.is_none() {
                    relation.from_revision_id = Some(revision.id.clone());
                }
            }
            if let Some(parent_id) = parent {
                all_relations.push(NewRelation {
                    kind: RelationKind::Revises,
                    from_item_id: item.id.clone(),
                    from_revision_id: Some(revision.id.clone()),
                    to_item_id: item.id.clone(),
                    to_revision_id: Some(parent_id),
                });
            }
            let assets = staged
                .iter()
                .map(|asset| NewAsset {
                    id: asset.asset_id.clone(),
                    revision_id: revision.id.clone(),
                    role: asset.role,
                    logical_path: asset.logical_path.as_str().to_owned(),
                    sha256: asset.sha256.clone(),
                    byte_size: asset.byte_size,
                    media_type: asset.media_type.clone(),
                    original_filename: asset.original_filename.clone(),
                })
                .collect();
            let duplicate = self.capture.repository.find_duplicate_text(
                &item.id,
                revision.text_sha256.as_ref().expect("text has hash"),
            )?;
            self.capture.persist_and_finalize(
                operation_id.clone(),
                operation,
                source,
                collection,
                item,
                revision,
                assets,
                all_relations,
                staged.iter().cloned().collect(),
                duplicate,
                false,
            )
        })();
        if result.is_err() {
            if let Some(asset) = &staged {
                let _ = self.capture.assets.discard_stage(asset);
            }
            let _ = self.capture.assets.complete_operation(&operation_id);
        }
        result.map_err(|error| error.with_operation(operation_id))
    }
}
