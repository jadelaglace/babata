use babata_domain::{
    AssetRole, CandidatePayload, CollectionId, ContentType, ItemId, Metadata, RevisionId,
    RevisionKind, Sha256, SourceId, SourceKind, SourceRouteId, TextPayload, UtcTimestamp,
};

use crate::{
    ApplicationError, CandidateCaptureCommand, CaptureFileCommand, CaptureImportCommand,
    CaptureOutcome, CaptureTextCommand,
    ports::{
        AssetStorePort, ClockPort, NewAsset, NewCollection, NewItem, NewRevision, NewSource,
        PersistGraph, RawRepositoryPort, StagedAsset,
    },
};

pub struct CaptureService<R, A, C> {
    pub(crate) repository: R,
    pub(crate) assets: A,
    pub(crate) clock: C,
}

impl<R, A, C> CaptureService<R, A, C>
where
    R: RawRepositoryPort,
    A: AssetStorePort,
    C: ClockPort,
{
    pub fn new(repository: R, assets: A, clock: C) -> Self {
        Self {
            repository,
            assets,
            clock,
        }
    }

    pub fn capture_text(
        &self,
        command: CaptureTextCommand,
    ) -> Result<CaptureOutcome, ApplicationError> {
        let text = TextPayload::new(command.text)?;
        let hash = text.hash();
        let identity = command
            .identity
            .or_else(|| command.native_id.as_ref().map(|id| format!("native:{id}")))
            .unwrap_or_else(|| format!("text:{}", hash.as_str()));
        self.capture_external(
            command.provider,
            command.context,
            command.locator,
            command.native_id,
            identity,
            ContentType::Text,
            command.metadata,
            command.source_published_at,
            Some(text.as_str().to_owned()),
            Some(hash),
            Vec::new(),
            RevisionKind::Capture,
        )
    }

    pub fn capture_file(
        &self,
        command: CaptureFileCommand,
    ) -> Result<CaptureOutcome, ApplicationError> {
        self.capture_file_like(command, AssetRole::Original, RevisionKind::Capture)
    }

    pub fn capture_export(
        &self,
        command: CaptureFileCommand,
    ) -> Result<CaptureOutcome, ApplicationError> {
        self.capture_file_like(command, AssetRole::Export, RevisionKind::Import)
    }

    pub fn capture_import(
        &self,
        command: CaptureImportCommand,
    ) -> Result<CaptureOutcome, ApplicationError> {
        let text = TextPayload::new(command.text)?;
        let route_evidence = command.route_evidence.clone();
        let operation_id = format!("op_{}", ulid::Ulid::new());
        let mut staged_assets = Vec::with_capacity(command.assets.len());
        for asset in &command.assets {
            match self.assets.stage(&asset.path, asset.role, &operation_id) {
                Ok(staged) => staged_assets.push(staged),
                Err(error) => {
                    for staged in &staged_assets {
                        let _ = self.assets.discard_stage(staged);
                    }
                    return Err(error);
                }
            }
        }
        let identity = command
            .identity
            .or_else(|| command.native_id.as_ref().map(|id| format!("native:{id}")))
            .unwrap_or_else(|| format!("text:{}", text.hash().as_str()));
        let result = self.capture_external(
            command.provider,
            command.context,
            command.locator,
            command.native_id,
            identity,
            command.content_type,
            command.metadata,
            command.source_published_at,
            Some(text.as_str().to_owned()),
            Some(text.hash()),
            staged_assets.clone(),
            RevisionKind::Import,
        );
        if result.is_err() {
            for asset in &staged_assets {
                let _ = self.assets.discard_stage(asset);
            }
        }
        if let (Ok(outcome), Some(evidence)) = (&result, route_evidence) {
            self.repository
                .record_route_evidence(&crate::ports::NewRouteEvidence {
                    route_id: evidence.route_id.0,
                    authorization_id: evidence.authorization_id,
                    source_reference: evidence.source_reference,
                    item_id: outcome.item_id.clone(),
                    revision_id: outcome.revision_id.clone(),
                    coverage: evidence.coverage,
                    reimported: outcome.reimported,
                    recorded_at: self.clock.now(),
                })?;
        }
        result
    }

    pub fn capture_candidate(
        &self,
        command: CandidateCaptureCommand,
    ) -> Result<CaptureOutcome, ApplicationError> {
        let candidate = command.candidate;
        if candidate.protocol_version != "1" {
            return Err(ApplicationError::Conflict(
                "unsupported candidate protocol version".to_owned(),
            ));
        }
        if candidate.route_id != SourceRouteId("source.browser".to_owned()) {
            return Err(ApplicationError::Conflict(
                "candidate route is not enabled for capture".to_owned(),
            ));
        }
        if candidate.content_type != ContentType::WebPage {
            return Err(ApplicationError::Conflict(
                "browser candidates must declare web_page content".to_owned(),
            ));
        }
        if candidate.source_reference.trim().is_empty() {
            return Err(ApplicationError::Domain(
                babata_domain::DomainError::Empty {
                    field: "source_reference",
                },
            ));
        }
        let CandidatePayload::Text { text } = candidate.payload;
        let payload = TextPayload::new(text)?;
        if payload.hash() != candidate.payload_sha256 {
            return Err(ApplicationError::Integrity(
                "candidate payload hash does not match its text".to_owned(),
            ));
        }
        self.capture_external(
            "browser".to_owned(),
            candidate.context,
            Some(candidate.source_reference.clone()),
            candidate.native_id,
            format!("browser:{}", candidate.source_reference),
            candidate.content_type,
            candidate.metadata,
            None,
            Some(payload.as_str().to_owned()),
            Some(payload.hash()),
            Vec::new(),
            RevisionKind::Capture,
        )
    }

    fn capture_file_like(
        &self,
        command: CaptureFileCommand,
        role: AssetRole,
        kind: RevisionKind,
    ) -> Result<CaptureOutcome, ApplicationError> {
        let operation_id = format!("op_{}", ulid::Ulid::new());
        let staged = self.assets.stage(&command.path, role, &operation_id)?;
        let identity = command
            .identity
            .or_else(|| command.native_id.as_ref().map(|id| format!("native:{id}")))
            .unwrap_or_else(|| format!("file:{}", staged.sha256.as_str()));
        let content_type = content_type_for(&command.path);
        let result = self.capture_external(
            command.provider,
            command.context,
            command.locator,
            command.native_id,
            identity,
            content_type,
            command.metadata,
            command.source_published_at,
            None,
            None,
            vec![staged.clone()],
            kind,
        );
        if result.is_err() {
            let _ = self.assets.discard_stage(&staged);
        }
        result
    }

    #[allow(clippy::too_many_arguments)]
    fn capture_external(
        &self,
        provider: String,
        context: Option<String>,
        locator: Option<String>,
        native_id: Option<String>,
        identity: String,
        content_type: ContentType,
        metadata: Metadata,
        source_published_at: Option<UtcTimestamp>,
        raw_text: Option<String>,
        text_sha256: Option<Sha256>,
        staged_assets: Vec<StagedAsset>,
        new_kind: RevisionKind,
    ) -> Result<CaptureOutcome, ApplicationError> {
        let now = self.clock.now();
        let source = match self
            .repository
            .find_source(SourceKind::External, &provider, None)?
        {
            Some(source) => source,
            None => NewSource {
                id: SourceId::new(),
                kind: SourceKind::External,
                provider,
                account_or_workspace: None,
                created_at: now.clone(),
            },
        };
        let collection = collection_from_context(&source, context, now.clone())?;
        let known = self
            .repository
            .find_by_source_identity(&source.id, &identity)?;
        let (item, parent, kind, ordinal) = if let Some((item, revision)) = known {
            let ordinal = self.repository.next_ordinal(&item.id)?;
            (item, Some(revision.id), RevisionKind::Import, ordinal)
        } else {
            (
                NewItem {
                    id: ItemId::new(),
                    source_id: source.id.clone(),
                    source_native_id: native_id,
                    source_locator: locator,
                    source_identity_key: Some(identity),
                    content_type,
                    source_published_at,
                    first_captured_at: now.clone(),
                    metadata,
                },
                None,
                new_kind,
                1,
            )
        };
        let duplicate_of = text_sha256
            .as_ref()
            .map(|hash| self.repository.find_duplicate_text(&item.id, hash))
            .transpose()?
            .flatten();
        let reimported = parent.is_some();
        let revision = NewRevision {
            id: RevisionId::new(),
            item_id: item.id.clone(),
            parent_revision_id: parent,
            kind,
            ordinal,
            captured_at: now.clone(),
            authored_at: None,
            revision_note: None,
            raw_text,
            text_sha256,
            metadata: Metadata::empty(),
        };
        let assets = staged_assets
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
        self.persist_and_finalize(
            source,
            collection,
            item,
            revision,
            assets,
            Vec::new(),
            staged_assets,
            duplicate_of,
            reimported,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn persist_and_finalize(
        &self,
        source: NewSource,
        collection: Option<NewCollection>,
        item: NewItem,
        revision: NewRevision,
        assets: Vec<NewAsset>,
        relations: Vec<crate::ports::NewRelation>,
        staged_assets: Vec<StagedAsset>,
        duplicate_of: Option<RevisionId>,
        reimported: bool,
    ) -> Result<CaptureOutcome, ApplicationError> {
        let graph = PersistGraph {
            source,
            collection,
            item: item.clone(),
            revision: revision.clone(),
            assets,
            relations,
        };
        self.repository.insert_capture_graph(&graph)?;
        for asset in &staged_assets {
            if let Err(error) = self.assets.finalize(asset) {
                self.repository.quarantine(&revision.id)?;
                for finalized in &staged_assets {
                    let _ = self
                        .assets
                        .quarantine_finalized(finalized, &format!("op_{}", ulid::Ulid::new()));
                }
                return Err(error);
            }
        }
        self.repository.mark_ready(&revision.id)?;
        for asset in &staged_assets {
            self.assets.discard_stage(asset)?;
        }
        let _detail = self.repository.load_detail(&item.id)?;
        Ok(CaptureOutcome {
            operation_id: format!("op_{}", ulid::Ulid::new()),
            item_id: item.id,
            revision_id: revision.id,
            asset_ids: staged_assets
                .into_iter()
                .map(|asset| asset.asset_id)
                .collect(),
            status: "ready".to_owned(),
            duplicate_of,
            reimported,
            warnings: Vec::new(),
        })
    }
}

pub(crate) fn collection_from_context(
    source: &NewSource,
    context: Option<String>,
    observed_at: UtcTimestamp,
) -> Result<Option<NewCollection>, ApplicationError> {
    context
        .map(|context| {
            let context = context.trim();
            if context.is_empty() {
                return Err(ApplicationError::Domain(
                    babata_domain::DomainError::Empty { field: "context" },
                ));
            }
            Ok(NewCollection {
                id: CollectionId::new(),
                source_id: source.id.clone(),
                native_id: context.to_owned(),
                collection_kind: "context".to_owned(),
                title: context.to_owned(),
                observed_at,
                metadata: Metadata::empty(),
            })
        })
        .transpose()
}

fn content_type_for(path: &str) -> ContentType {
    match path
        .rsplit('.')
        .next()
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("txt" | "md" | "rst") => ContentType::Text,
        Some("pdf" | "doc" | "docx" | "ppt" | "pptx" | "xls" | "xlsx") => ContentType::Document,
        Some("jpg" | "jpeg" | "png" | "gif" | "webp") => ContentType::Image,
        Some("mp3" | "wav" | "m4a" | "flac") => ContentType::Audio,
        Some("mp4" | "mov" | "mkv" | "webm") => ContentType::Video,
        Some("html" | "htm") => ContentType::WebPage,
        Some("zip" | "7z" | "tar" | "gz") => ContentType::Archive,
        _ => ContentType::Unknown,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    #[derive(Debug, Default, Clone, Copy)]
    pub(crate) struct FixedClock;

    impl ClockPort for FixedClock {
        fn now(&self) -> UtcTimestamp {
            UtcTimestamp::parse("2026-01-01T00:00:00Z").unwrap()
        }
    }
    use crate::{
        CaptureFileCommand,
        ports::{AssetStorePort, NewRelation, RawRepositoryPort},
    };
    use babata_domain::{AssetId, AssetRole, LogicalPath};

    #[derive(Clone, Default)]
    pub(crate) struct MockRepository {
        pub(crate) state: Arc<Mutex<State>>,
    }
    #[derive(Default)]
    pub(crate) struct State {
        pub(crate) sources: Vec<NewSource>,
        pub(crate) items: Vec<NewItem>,
        pub(crate) revisions: Vec<NewRevision>,
        pub(crate) relations: Vec<NewRelation>,
        pub(crate) quarantined: Vec<RevisionId>,
        pub(crate) route_evidence: Vec<babata_domain::RouteEvidence>,
    }
    impl RawRepositoryPort for MockRepository {
        fn find_source(
            &self,
            kind: SourceKind,
            provider: &str,
            account: Option<&str>,
        ) -> Result<Option<NewSource>, ApplicationError> {
            Ok(self
                .state
                .lock()
                .unwrap()
                .sources
                .iter()
                .find(|source| {
                    source.kind == kind
                        && source.provider == provider
                        && source.account_or_workspace.as_deref() == account
                })
                .cloned())
        }
        fn find_item(&self, id: &ItemId) -> Result<Option<NewItem>, ApplicationError> {
            Ok(self
                .state
                .lock()
                .unwrap()
                .items
                .iter()
                .find(|item| &item.id == id)
                .cloned())
        }
        fn find_revision(&self, id: &RevisionId) -> Result<Option<NewRevision>, ApplicationError> {
            Ok(self
                .state
                .lock()
                .unwrap()
                .revisions
                .iter()
                .find(|revision| &revision.id == id)
                .cloned())
        }
        fn find_by_source_identity(
            &self,
            source: &SourceId,
            identity: &str,
        ) -> Result<Option<(NewItem, NewRevision)>, ApplicationError> {
            let state = self.state.lock().unwrap();
            Ok(state
                .items
                .iter()
                .find(|item| {
                    &item.source_id == source
                        && item.source_identity_key.as_deref() == Some(identity)
                })
                .and_then(|item| {
                    state
                        .revisions
                        .iter()
                        .filter(|revision| revision.item_id == item.id)
                        .max_by_key(|revision| revision.ordinal)
                        .cloned()
                        .map(|revision| (item.clone(), revision))
                }))
        }
        fn next_ordinal(&self, item: &ItemId) -> Result<u32, ApplicationError> {
            Ok(self
                .state
                .lock()
                .unwrap()
                .revisions
                .iter()
                .filter(|revision| &revision.item_id == item)
                .map(|revision| revision.ordinal)
                .max()
                .unwrap_or(0)
                + 1)
        }
        fn find_duplicate_text(
            &self,
            item: &ItemId,
            hash: &Sha256,
        ) -> Result<Option<RevisionId>, ApplicationError> {
            Ok(self
                .state
                .lock()
                .unwrap()
                .revisions
                .iter()
                .filter(|revision| {
                    &revision.item_id == item && revision.text_sha256.as_ref() == Some(hash)
                })
                .map(|revision| revision.id.clone())
                .next())
        }
        fn insert_capture_graph(&self, graph: &PersistGraph) -> Result<(), ApplicationError> {
            let mut state = self.state.lock().unwrap();
            if !state
                .sources
                .iter()
                .any(|source| source.id == graph.source.id)
            {
                state.sources.push(graph.source.clone());
            }
            if !state.items.iter().any(|item| item.id == graph.item.id) {
                state.items.push(graph.item.clone());
            }
            state.revisions.push(graph.revision.clone());
            state.relations.extend(graph.relations.clone());
            Ok(())
        }
        fn mark_ready(&self, _: &RevisionId) -> Result<(), ApplicationError> {
            Ok(())
        }
        fn quarantine(&self, revision_id: &RevisionId) -> Result<(), ApplicationError> {
            self.state
                .lock()
                .unwrap()
                .quarantined
                .push(revision_id.clone());
            Ok(())
        }
        fn load_detail(&self, item_id: &ItemId) -> Result<crate::RecordDetail, ApplicationError> {
            Ok(crate::RecordDetail {
                item_id: item_id.clone(),
                source_kind: SourceKind::External,
                provider: "mock".to_owned(),
                content_type: ContentType::Text,
                revisions: Vec::new(),
                assets: Vec::new(),
                relations: Vec::new(),
            })
        }
        fn record_route_evidence(
            &self,
            evidence: &crate::ports::NewRouteEvidence,
        ) -> Result<(), ApplicationError> {
            self.state
                .lock()
                .unwrap()
                .route_evidence
                .push(babata_domain::RouteEvidence {
                    route_id: SourceRouteId(evidence.route_id.clone()),
                    authorization_id: evidence.authorization_id.clone(),
                    source_reference: evidence.source_reference.clone(),
                    item_id: evidence.item_id.clone(),
                    revision_id: evidence.revision_id.clone(),
                    coverage: babata_domain::RouteCoverage {
                        metadata: evidence.coverage.metadata,
                        attachments: evidence.coverage.attachments,
                        revisions: evidence.coverage.revisions,
                        limitations: evidence.coverage.limitations.clone(),
                    },
                    reimported: evidence.reimported,
                    recorded_at: evidence.recorded_at.clone(),
                });
            Ok(())
        }
        fn route_evidence(
            &self,
            route_id: &str,
        ) -> Result<Vec<babata_domain::RouteEvidence>, ApplicationError> {
            Ok(self
                .state
                .lock()
                .unwrap()
                .route_evidence
                .iter()
                .filter(|evidence| evidence.route_id.0 == route_id)
                .cloned()
                .collect())
        }
    }
    #[derive(Clone, Default)]
    pub(crate) struct MockAssets {
        pub(crate) fail_stage: bool,
        pub(crate) fail_finalize: bool,
        finalized: Arc<Mutex<u32>>,
        discarded: Arc<Mutex<u32>>,
    }
    impl AssetStorePort for MockAssets {
        fn stage(
            &self,
            _: &str,
            role: AssetRole,
            _: &str,
        ) -> Result<StagedAsset, ApplicationError> {
            if self.fail_stage {
                return Err(ApplicationError::Asset("staging failed".to_owned()));
            }
            Ok(StagedAsset {
                asset_id: AssetId::new(),
                role,
                staging_key: "test".to_owned(),
                logical_path: LogicalPath::parse("01_raw/assets/test").unwrap(),
                sha256: Sha256::of_bytes(b"test"),
                byte_size: 4,
                media_type: None,
                original_filename: None,
            })
        }
        fn finalize(&self, _: &StagedAsset) -> Result<(), ApplicationError> {
            if self.fail_finalize {
                return Err(ApplicationError::Asset("finalization failed".to_owned()));
            }
            *self.finalized.lock().unwrap() += 1;
            Ok(())
        }
        fn hash(&self, _: &str) -> Result<Sha256, ApplicationError> {
            Ok(Sha256::of_bytes(b"test"))
        }
        fn open(&self, _: &LogicalPath) -> Result<Vec<u8>, ApplicationError> {
            Ok(b"test".to_vec())
        }
        fn verify(&self, _: &StagedAsset) -> Result<bool, ApplicationError> {
            Ok(true)
        }
        fn discard_stage(&self, _: &StagedAsset) -> Result<(), ApplicationError> {
            *self.discarded.lock().unwrap() += 1;
            Ok(())
        }
        fn quarantine_finalized(&self, _: &StagedAsset, _: &str) -> Result<(), ApplicationError> {
            Ok(())
        }
    }
    fn text(provider: &str, value: &str) -> CaptureTextCommand {
        CaptureTextCommand {
            provider: provider.to_owned(),
            text: value.to_owned(),
            context: None,
            locator: None,
            native_id: None,
            identity: None,
            metadata: Metadata::empty(),
            source_published_at: None,
        }
    }

    #[test]
    fn new_text_capture_creates_ready_outcome() {
        let service =
            CaptureService::new(MockRepository::default(), MockAssets::default(), FixedClock);
        assert_eq!(
            service
                .capture_text(text("fixture", "hello"))
                .unwrap()
                .status,
            "ready"
        );
    }
    #[test]
    fn identical_reimport_is_linked_not_suppressed() {
        let repository = MockRepository::default();
        let service = CaptureService::new(repository.clone(), MockAssets::default(), FixedClock);
        let first = service.capture_text(text("fixture", "hello")).unwrap();
        let second = service.capture_text(text("fixture", "hello")).unwrap();
        assert_eq!(first.item_id, second.item_id);
        assert_ne!(first.revision_id, second.revision_id);
        assert_eq!(second.duplicate_of, Some(first.revision_id));
    }
    #[test]
    fn empty_text_is_rejected_before_write() {
        assert!(
            CaptureService::new(MockRepository::default(), MockAssets::default(), FixedClock)
                .capture_text(text("fixture", " "))
                .is_err()
        );
    }
    #[test]
    fn failed_file_stage_does_not_persist() {
        let repository = MockRepository::default();
        let service = CaptureService::new(
            repository.clone(),
            MockAssets {
                fail_stage: true,
                ..Default::default()
            },
            FixedClock,
        );
        assert!(
            service
                .capture_file(CaptureFileCommand {
                    provider: "fixture".to_owned(),
                    path: "missing.txt".to_owned(),
                    context: None,
                    locator: None,
                    native_id: None,
                    identity: None,
                    metadata: Metadata::empty(),
                    source_published_at: None
                })
                .is_err()
        );
        assert!(repository.state.lock().unwrap().revisions.is_empty());
    }

    #[test]
    fn failed_asset_finalization_quarantines_the_pending_revision() {
        let repository = MockRepository::default();
        let service = CaptureService::new(
            repository.clone(),
            MockAssets {
                fail_finalize: true,
                ..Default::default()
            },
            FixedClock,
        );
        assert!(
            service
                .capture_file(CaptureFileCommand {
                    provider: "fixture".to_owned(),
                    path: "fixture.txt".to_owned(),
                    context: None,
                    locator: None,
                    native_id: None,
                    identity: None,
                    metadata: Metadata::empty(),
                    source_published_at: None,
                })
                .is_err()
        );
        let state = repository.state.lock().unwrap();
        assert_eq!(state.revisions.len(), 1);
        assert_eq!(state.quarantined, vec![state.revisions[0].id.clone()]);
    }
}
