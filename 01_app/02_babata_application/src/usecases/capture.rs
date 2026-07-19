use babata_domain::{
    AssetRole, CandidateEnvelope, CandidatePayload, CollectionId, ContentType, ItemId, Metadata,
    RawState, RevisionId, RevisionKind, Sha256, SourceId, SourceKind, TextPayload, UtcTimestamp,
};

use crate::{
    ApplicationError, AttachRecoveredAssetsCommand, CandidateCaptureCommand, CaptureFileCommand,
    CaptureImportCommand, CaptureOutcome, CaptureTextCommand,
    ports::{
        AssetStorePort, ClockPort, FinalizeAssetOutcome, NewAsset, NewCaptureOperation,
        NewCollection, NewItem, NewRevision, NewSource, PersistGraph, RawRepositoryPort,
        StagedAsset,
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
        validate_provider(&command.provider)?;
        let text = TextPayload::new(command.text)?;
        let hash = text.hash();
        let identity = command
            .identity
            .or_else(|| command.native_id.as_ref().map(|id| format!("native:{id}")))
            .unwrap_or_else(|| format!("text:{}", hash.as_str()));
        let operation_id = new_operation_id();
        self.capture_external(
            operation_id.clone(),
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
        .map_err(|error| error.with_operation(operation_id))
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
        validate_provider(&command.provider)?;
        let text = TextPayload::new(command.text)?;
        let route_evidence = command.route_evidence.clone();
        let operation_id = new_operation_id();
        let staged_assets = self.stage_import_assets(&command.assets, &operation_id)?;
        let identity = command
            .identity
            .or_else(|| command.native_id.as_ref().map(|id| format!("native:{id}")))
            .unwrap_or_else(|| format!("text:{}", text.hash().as_str()));
        let result = self.capture_external(
            operation_id.clone(),
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
            let _ = self.assets.complete_operation(&operation_id);
        }
        let mut outcome = result.map_err(|error| error.with_operation(operation_id.clone()))?;
        if let Some(evidence) = route_evidence {
            if self
                .repository
                .record_route_evidence(&crate::ports::NewRouteEvidence {
                    route_id: evidence.route_id.0,
                    authorization_id: evidence.authorization_id,
                    source_reference: evidence.source_reference,
                    item_id: outcome.item_id.clone(),
                    revision_id: outcome.revision_id.clone(),
                    coverage: evidence.coverage,
                    reimported: outcome.reimported,
                    recorded_at: self.clock.now(),
                })
                .is_err()
            {
                outcome
                    .warnings
                    .push("route evidence persistence is pending".to_owned());
            }
        }
        Ok(outcome)
    }

    /// Append recovered source/preview assets as a new C0 revision. The parent
    /// revision and all existing bytes remain immutable.
    pub fn attach_recovered_assets(
        &self,
        command: AttachRecoveredAssetsCommand,
    ) -> Result<CaptureOutcome, ApplicationError> {
        if command.assets.is_empty() {
            return Err(ApplicationError::Integrity(
                "attach-assets needs at least one file".to_owned(),
            ));
        }
        if command.reason.trim().is_empty() {
            return Err(ApplicationError::Integrity(
                "attach-assets needs a non-empty reason".to_owned(),
            ));
        }
        let parent = self
            .repository
            .find_revision(&command.revision_id)?
            .ok_or_else(|| {
                ApplicationError::NotFound(format!("revision {}", command.revision_id))
            })?;
        let state = self
            .repository
            .find_revision_state(&command.revision_id)?
            .ok_or_else(|| {
                ApplicationError::NotFound(format!("revision {}", command.revision_id))
            })?;
        if state != RawState::Ready {
            return Err(ApplicationError::Integrity(format!(
                "revision {} is {state:?}; recovered assets require a ready parent",
                command.revision_id
            )));
        }
        let item = self
            .repository
            .find_item(&parent.item_id)?
            .ok_or_else(|| ApplicationError::NotFound(format!("item {}", parent.item_id)))?;
        let source = self
            .repository
            .find_source_by_id(&item.source_id)?
            .ok_or_else(|| ApplicationError::NotFound(format!("source {}", item.source_id)))?;

        // Resolve every fallible repository value before staging files so an
        // early database failure cannot leave an untracked staging operation.
        let ordinal = self.repository.next_ordinal(&item.id)?;
        let operation_id = new_operation_id();
        let staged_assets = self.stage_import_assets(&command.assets, &operation_id)?;

        let now = self.clock.now();
        let revision = NewRevision {
            id: RevisionId::new(),
            item_id: item.id.clone(),
            parent_revision_id: Some(parent.id.clone()),
            kind: RevisionKind::Import,
            ordinal,
            captured_at: now.clone(),
            authored_at: parent.authored_at,
            revision_note: Some(command.reason.trim().to_owned()),
            raw_text: parent.raw_text,
            text_sha256: parent.text_sha256.clone(),
            metadata: parent.metadata,
        };
        let operation = NewCaptureOperation {
            operation_id: operation_id.clone(),
            item_id: item.id.clone(),
            revision_id: revision.id.clone(),
            source_native_id: item.source_native_id.clone(),
            source_locator: item.source_locator.clone(),
            source_published_at: item.source_published_at.clone(),
            metadata: command.metadata,
            started_at: now,
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
        let duplicate_of = revision.text_sha256.as_ref().map(|_| parent.id);
        let result = self.persist_and_finalize(
            operation_id.clone(),
            operation,
            source,
            None,
            item,
            revision,
            assets,
            Vec::new(),
            staged_assets.clone(),
            duplicate_of,
            true,
        );
        if result.is_err() {
            for asset in &staged_assets {
                let _ = self.assets.discard_stage(asset);
            }
            let _ = self.assets.complete_operation(&operation_id);
        }
        result.map_err(|error| error.with_operation(operation_id))
    }

    fn stage_import_assets(
        &self,
        assets: &[crate::CaptureImportAsset],
        operation_id: &str,
    ) -> Result<Vec<StagedAsset>, ApplicationError> {
        let mut staged_assets = Vec::with_capacity(assets.len());
        for asset in assets {
            match self.assets.stage(&asset.path, asset.role, operation_id) {
                Ok(staged) => staged_assets.push(staged),
                Err(error) => {
                    for staged in &staged_assets {
                        let _ = self.assets.discard_stage(staged);
                    }
                    let _ = self.assets.complete_operation(operation_id);
                    return Err(error.with_operation(operation_id.to_owned()));
                }
            }
        }
        Ok(staged_assets)
    }

    pub fn capture_candidate(
        &self,
        command: CandidateCaptureCommand,
    ) -> Result<CaptureOutcome, ApplicationError> {
        let assets = command.assets;
        let candidate = command.candidate;
        validate_candidate(&candidate)?;
        if command
            .route_evidence
            .as_ref()
            .is_some_and(|evidence| evidence.route_id != candidate.route_id)
        {
            return Err(ApplicationError::Conflict(
                "route evidence does not match candidate route".to_owned(),
            ));
        }
        let CandidatePayload::Text { text } = candidate.payload;
        let payload = TextPayload::new(text)?;
        if payload.hash() != candidate.payload_sha256 {
            return Err(ApplicationError::Integrity(
                "candidate payload hash does not match its text".to_owned(),
            ));
        }
        let provider = candidate
            .route_id
            .0
            .strip_prefix("source.")
            .unwrap_or(&candidate.route_id.0)
            .to_owned();
        let route_evidence = command.route_evidence;
        let operation_id = new_operation_id();
        let mut staged_assets = Vec::with_capacity(assets.len());
        for asset in &assets {
            match self.assets.stage(&asset.path, asset.role, &operation_id) {
                Ok(staged) => staged_assets.push(staged),
                Err(error) => {
                    for staged in &staged_assets {
                        let _ = self.assets.discard_stage(staged);
                    }
                    let _ = self.assets.complete_operation(&operation_id);
                    return Err(error.with_operation(operation_id));
                }
            }
        }
        let result = self.capture_external(
            operation_id.clone(),
            provider.clone(),
            candidate.context,
            Some(candidate.source_reference.clone()),
            candidate.native_id,
            format!("{provider}:{}", candidate.source_reference),
            candidate.content_type,
            candidate.metadata,
            None,
            Some(payload.as_str().to_owned()),
            Some(payload.hash()),
            staged_assets.clone(),
            RevisionKind::Capture,
        );
        if result.is_err() {
            for staged in &staged_assets {
                let _ = self.assets.discard_stage(staged);
            }
            let _ = self.assets.complete_operation(&operation_id);
        }
        let mut outcome = result.map_err(|error| error.with_operation(operation_id))?;
        if let Some(evidence) = route_evidence {
            if self
                .repository
                .record_route_evidence(&crate::ports::NewRouteEvidence {
                    route_id: evidence.route_id.0,
                    authorization_id: evidence.authorization_id,
                    source_reference: evidence.source_reference,
                    item_id: outcome.item_id.clone(),
                    revision_id: outcome.revision_id.clone(),
                    coverage: evidence.coverage,
                    reimported: outcome.reimported,
                    recorded_at: self.clock.now(),
                })
                .is_err()
            {
                outcome
                    .warnings
                    .push("route evidence persistence is pending".to_owned());
            }
        }
        Ok(outcome)
    }

    fn capture_file_like(
        &self,
        command: CaptureFileCommand,
        role: AssetRole,
        kind: RevisionKind,
    ) -> Result<CaptureOutcome, ApplicationError> {
        validate_provider(&command.provider)?;
        let operation_id = new_operation_id();
        let staged = match self.assets.stage(&command.path, role, &operation_id) {
            Ok(staged) => staged,
            Err(error) => {
                let _ = self.assets.complete_operation(&operation_id);
                return Err(error.with_operation(operation_id));
            }
        };
        let identity = command
            .identity
            .or_else(|| command.native_id.as_ref().map(|id| format!("native:{id}")))
            .unwrap_or_else(|| format!("file:{}", staged.sha256.as_str()));
        let content_type = content_type_for(&command.path);
        let result = self.capture_external(
            operation_id.clone(),
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
            let _ = self.assets.complete_operation(&operation_id);
        }
        result.map_err(|error| error.with_operation(operation_id))
    }

    #[allow(clippy::too_many_arguments)]
    fn capture_external(
        &self,
        operation_id: String,
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
        let operation_native_id = native_id.clone();
        let operation_locator = locator.clone();
        let operation_published_at = source_published_at.clone();
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
                    metadata: metadata.clone(),
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
            metadata: metadata.clone(),
        };
        let operation = NewCaptureOperation {
            operation_id: operation_id.clone(),
            item_id: item.id.clone(),
            revision_id: revision.id.clone(),
            source_native_id: operation_native_id,
            source_locator: operation_locator,
            source_published_at: operation_published_at,
            metadata,
            started_at: now,
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
            operation_id,
            operation,
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
        operation_id: String,
        operation: NewCaptureOperation,
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
        self.assets.begin_operation(&operation_id)?;
        let graph = PersistGraph {
            operation,
            source,
            collection,
            item: item.clone(),
            revision: revision.clone(),
            assets,
            relations,
        };
        if let Err(error) = self.repository.insert_capture_graph(&graph) {
            for asset in &staged_assets {
                let _ = self.assets.discard_stage(asset);
            }
            let _ = self.assets.complete_operation(&operation_id);
            return Err(error);
        }
        let mut finalized = Vec::with_capacity(staged_assets.len());
        for asset in &staged_assets {
            match self.assets.finalize(asset) {
                Ok(outcome) => finalized.push((asset.clone(), outcome)),
                Err(error) => {
                    self.preserve_failed_capture(&operation_id, &revision.id, &finalized, &error);
                    return Err(error);
                }
            }
        }
        for asset in &staged_assets {
            match self.assets.verify(asset) {
                Ok(true) => {}
                Ok(false) => {
                    let error = ApplicationError::Integrity(
                        "finalized asset failed hash verification".to_owned(),
                    );
                    self.preserve_failed_capture(&operation_id, &revision.id, &finalized, &error);
                    return Err(error);
                }
                Err(error) => {
                    self.preserve_failed_capture(&operation_id, &revision.id, &finalized, &error);
                    return Err(error);
                }
            }
        }
        if let Err(error) = self.repository.mark_ready(&revision.id) {
            self.preserve_failed_capture(&operation_id, &revision.id, &finalized, &error);
            return Err(error);
        }
        let mut warnings = Vec::new();
        for asset in &staged_assets {
            if self.assets.discard_stage(asset).is_err() {
                warnings.push("recovery staging cleanup is pending".to_owned());
                break;
            }
        }
        if self.assets.complete_operation(&operation_id).is_err() {
            warnings.push("recovery journal cleanup is pending".to_owned());
        }
        let detail = self
            .repository
            .load_detail(&item.id)
            .ok()
            .and_then(|detail| {
                if validate_ready_readback(&detail, &revision.id, &staged_assets).is_ok() {
                    Some(detail)
                } else {
                    None
                }
            });
        if detail.is_none() {
            warnings.push("ready capture committed but repository read-back failed".to_owned());
        }
        Ok(CaptureOutcome {
            operation_id,
            item_id: item.id,
            revision_id: revision.id,
            asset_ids: staged_assets
                .into_iter()
                .map(|asset| asset.asset_id)
                .collect(),
            status: "ready".to_owned(),
            duplicate_of,
            reimported,
            warnings,
            record: detail,
        })
    }

    fn preserve_failed_capture(
        &self,
        operation_id: &str,
        revision_id: &RevisionId,
        finalized: &[(StagedAsset, FinalizeAssetOutcome)],
        error: &ApplicationError,
    ) {
        let _ =
            self.assets
                .preserve_operation(operation_id, &revision_id.to_string(), error.code());
        for (asset, outcome) in finalized {
            let _ = self
                .assets
                .quarantine_finalized(asset, operation_id, *outcome);
        }
        let _ = self.repository.quarantine(revision_id, error.code());
    }
}

fn validate_candidate(candidate: &CandidateEnvelope) -> Result<(), ApplicationError> {
    if candidate.protocol_version != "1" {
        return Err(ApplicationError::Conflict(
            "unsupported candidate protocol version".to_owned(),
        ));
    }
    if !matches!(
        candidate.route_id.0.as_str(),
        "source.feishu"
            | "source.kimi"
            | "source.zhihu"
            | "source.bilibili"
            | "source.xiaohongshu"
            | "source.douyin"
            | "source.doubao"
            | "source.chatgpt"
            | "source.yuque"
            | "source.wechat_articles"
            | "source.browser_pages"
            | "source.browser_bookmarks"
    ) {
        return Err(ApplicationError::Conflict(
            "candidate route is not enabled for capture".to_owned(),
        ));
    }
    match candidate.route_id.0.as_str() {
        "source.browser_pages" if candidate.content_type != ContentType::WebPage => {
            return Err(ApplicationError::Conflict(
                "browser page candidates must declare web_page content".to_owned(),
            ));
        }
        "source.browser_bookmarks" if candidate.content_type != ContentType::Document => {
            return Err(ApplicationError::Conflict(
                "browser bookmark candidates must declare document content".to_owned(),
            ));
        }
        "source.wechat_articles" if candidate.content_type != ContentType::Document => {
            return Err(ApplicationError::Conflict(
                "WeChat article candidates must declare document content".to_owned(),
            ));
        }
        _ => {}
    }
    if candidate.route_id.0 == "source.feishu" && candidate.content_type != ContentType::Document {
        return Err(ApplicationError::Conflict(
            "Feishu candidates must declare document content".to_owned(),
        ));
    }
    if matches!(
        candidate.route_id.0.as_str(),
        "source.kimi"
            | "source.zhihu"
            | "source.bilibili"
            | "source.xiaohongshu"
            | "source.douyin"
            | "source.doubao"
            | "source.chatgpt"
            | "source.yuque"
    ) && !matches!(
        candidate.content_type,
        ContentType::Document | ContentType::WebPage
    ) {
        return Err(ApplicationError::Conflict(
            "named browser-platform candidates must declare document or web_page content"
                .to_owned(),
        ));
    }
    if candidate.source_reference.trim().is_empty() {
        return Err(ApplicationError::Domain(
            babata_domain::DomainError::Empty {
                field: "source_reference",
            },
        ));
    }
    Ok(())
}

fn new_operation_id() -> String {
    format!("op_{}", ulid::Ulid::new())
}

fn validate_provider(provider: &str) -> Result<(), ApplicationError> {
    if provider.trim().is_empty() {
        return Err(babata_domain::DomainError::Empty { field: "provider" }.into());
    }
    Ok(())
}

fn validate_ready_readback(
    detail: &crate::RecordDetail,
    revision_id: &RevisionId,
    staged_assets: &[StagedAsset],
) -> Result<(), ApplicationError> {
    let revision = detail
        .revisions
        .iter()
        .find(|revision| &revision.revision_id == revision_id)
        .ok_or_else(|| {
            ApplicationError::Integrity("ready revision did not read back".to_owned())
        })?;
    if revision.state != RawState::Ready {
        return Err(ApplicationError::Integrity(
            "revision read back without ready state".to_owned(),
        ));
    }
    for staged in staged_assets {
        let asset = detail
            .assets
            .iter()
            .find(|asset| asset.asset_id == staged.asset_id)
            .ok_or_else(|| {
                ApplicationError::Integrity("ready asset did not read back".to_owned())
            })?;
        if asset.state != RawState::Ready || asset.sha256 != staged.sha256.as_str() {
            return Err(ApplicationError::Integrity(
                "asset read back with wrong state or hash".to_owned(),
            ));
        }
    }
    Ok(())
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
        ports::{AssetStorePort, NewAsset, NewRelation, RawRepositoryPort},
    };
    use babata_domain::{AssetId, AssetRole, LogicalPath, RouteCoverage, SourceRouteId};

    #[derive(Clone, Default)]
    pub(crate) struct MockRepository {
        pub(crate) state: Arc<Mutex<State>>,
        pub(crate) fail_mark_ready: bool,
        pub(crate) fail_next_ordinal: bool,
    }
    #[derive(Default)]
    pub(crate) struct State {
        pub(crate) sources: Vec<NewSource>,
        pub(crate) items: Vec<NewItem>,
        pub(crate) revisions: Vec<NewRevision>,
        pub(crate) operations: Vec<NewCaptureOperation>,
        pub(crate) assets: Vec<NewAsset>,
        pub(crate) relations: Vec<NewRelation>,
        pub(crate) quarantined: Vec<RevisionId>,
        pub(crate) states: Vec<(RevisionId, RawState)>,
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
        fn find_source_by_id(&self, id: &SourceId) -> Result<Option<NewSource>, ApplicationError> {
            Ok(self
                .state
                .lock()
                .unwrap()
                .sources
                .iter()
                .find(|source| &source.id == id)
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
        fn find_revision_state(
            &self,
            id: &RevisionId,
        ) -> Result<Option<RawState>, ApplicationError> {
            Ok(self
                .state
                .lock()
                .unwrap()
                .states
                .iter()
                .find(|(revision_id, _)| revision_id == id)
                .map(|(_, state)| *state))
        }
        fn find_asset(&self, asset_id: &AssetId) -> Result<Option<NewAsset>, ApplicationError> {
            Ok(self
                .state
                .lock()
                .unwrap()
                .assets
                .iter()
                .find(|asset| &asset.id == asset_id)
                .cloned())
        }
        fn list_assets_for_revision(
            &self,
            revision_id: &RevisionId,
        ) -> Result<Vec<NewAsset>, ApplicationError> {
            Ok(self
                .state
                .lock()
                .unwrap()
                .assets
                .iter()
                .filter(|asset| &asset.revision_id == revision_id)
                .cloned()
                .collect())
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
            if self.fail_next_ordinal {
                return Err(ApplicationError::Storage("next ordinal failed".to_owned()));
            }
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
            state.operations.push(graph.operation.clone());
            state.assets.extend(graph.assets.clone());
            state.relations.extend(graph.relations.clone());
            state
                .states
                .push((graph.revision.id.clone(), RawState::Pending));
            Ok(())
        }
        fn mark_ready(&self, revision_id: &RevisionId) -> Result<(), ApplicationError> {
            if self.fail_mark_ready {
                return Err(ApplicationError::Storage(
                    "ready transition failed".to_owned(),
                ));
            }
            if let Some((_, state)) = self
                .state
                .lock()
                .unwrap()
                .states
                .iter_mut()
                .find(|(id, _)| id == revision_id)
            {
                *state = RawState::Ready;
            }
            Ok(())
        }
        fn quarantine(&self, revision_id: &RevisionId, _: &str) -> Result<(), ApplicationError> {
            self.state
                .lock()
                .unwrap()
                .quarantined
                .push(revision_id.clone());
            if let Some((_, state)) = self
                .state
                .lock()
                .unwrap()
                .states
                .iter_mut()
                .find(|(id, _)| id == revision_id)
            {
                *state = RawState::Quarantined;
            }
            Ok(())
        }
        fn load_detail(&self, item_id: &ItemId) -> Result<crate::RecordDetail, ApplicationError> {
            let state = self.state.lock().unwrap();
            let item = state
                .items
                .iter()
                .find(|item| &item.id == item_id)
                .ok_or_else(|| ApplicationError::NotFound(item_id.to_string()))?;
            let source = state
                .sources
                .iter()
                .find(|source| source.id == item.source_id)
                .ok_or_else(|| ApplicationError::Integrity("mock source is missing".to_owned()))?;
            Ok(crate::RecordDetail {
                item_id: item_id.clone(),
                source_id: source.id.clone(),
                source_kind: source.kind,
                provider: source.provider.clone(),
                content_type: item.content_type,
                source_native_id: item.source_native_id.clone(),
                source_locator: item.source_locator.clone(),
                source_identity_key: item.source_identity_key.clone(),
                metadata: item.metadata.clone(),
                collections: Vec::new(),
                revisions: state
                    .revisions
                    .iter()
                    .filter(|revision| &revision.item_id == item_id)
                    .map(|revision| crate::RevisionDetail {
                        revision_id: revision.id.clone(),
                        parent_revision_id: revision.parent_revision_id.clone(),
                        kind: format!("{:?}", revision.kind).to_ascii_lowercase(),
                        ordinal: revision.ordinal,
                        captured_at: revision.captured_at.clone(),
                        authored_at: revision.authored_at.clone(),
                        revision_note: revision.revision_note.clone(),
                        raw_text: revision.raw_text.clone(),
                        text_sha256: revision.text_sha256.as_ref().map(ToString::to_string),
                        metadata: revision.metadata.clone(),
                        state: state
                            .states
                            .iter()
                            .find(|(id, _)| id == &revision.id)
                            .map_or(RawState::Pending, |(_, state)| *state),
                        provenance: state
                            .operations
                            .iter()
                            .find(|operation| operation.revision_id == revision.id)
                            .map(|operation| crate::CaptureProvenanceDetail {
                                operation_id: operation.operation_id.clone(),
                                source_native_id: operation.source_native_id.clone(),
                                source_locator: operation.source_locator.clone(),
                                source_published_at: operation.source_published_at.clone(),
                                metadata: operation.metadata.clone(),
                                state: state
                                    .states
                                    .iter()
                                    .find(|(id, _)| id == &revision.id)
                                    .map_or(RawState::Pending, |(_, state)| *state),
                                failure_code: None,
                            }),
                    })
                    .collect(),
                assets: state
                    .assets
                    .iter()
                    .filter(|asset| {
                        state.revisions.iter().any(|revision| {
                            revision.item_id == *item_id && revision.id == asset.revision_id
                        })
                    })
                    .map(|asset| crate::AssetDetail {
                        asset_id: asset.id.clone(),
                        role: asset.role,
                        logical_path: asset.logical_path.clone(),
                        sha256: asset.sha256.to_string(),
                        byte_size: asset.byte_size,
                        media_type: asset.media_type.clone(),
                        original_filename: asset.original_filename.clone(),
                        state: state
                            .states
                            .iter()
                            .find(|(id, _)| id == &asset.revision_id)
                            .map_or(RawState::Pending, |(_, state)| *state),
                    })
                    .collect(),
                relations: state
                    .relations
                    .iter()
                    .filter(|relation| {
                        &relation.from_item_id == item_id || &relation.to_item_id == item_id
                    })
                    .map(|relation| crate::RelationDetail {
                        kind: relation.kind,
                        from_item_id: relation.from_item_id.clone(),
                        from_revision_id: relation.from_revision_id.clone(),
                        to_item_id: relation.to_item_id.clone(),
                        to_revision_id: relation.to_revision_id.clone(),
                    })
                    .collect(),
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
        pub(crate) fail_verify: bool,
        staged: Arc<Mutex<u32>>,
        finalized: Arc<Mutex<u32>>,
        discarded: Arc<Mutex<u32>>,
        recovery_markers: Arc<Mutex<u32>>,
    }
    impl AssetStorePort for MockAssets {
        fn begin_operation(&self, _: &str) -> Result<(), ApplicationError> {
            Ok(())
        }
        fn preserve_operation(&self, _: &str, _: &str, _: &str) -> Result<(), ApplicationError> {
            Ok(())
        }
        fn complete_operation(&self, _: &str) -> Result<(), ApplicationError> {
            Ok(())
        }
        fn stage(
            &self,
            _: &str,
            role: AssetRole,
            _: &str,
        ) -> Result<StagedAsset, ApplicationError> {
            if self.fail_stage {
                return Err(ApplicationError::Asset("staging failed".to_owned()));
            }
            *self.staged.lock().unwrap() += 1;
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
        fn finalize(&self, _: &StagedAsset) -> Result<FinalizeAssetOutcome, ApplicationError> {
            if self.fail_finalize {
                return Err(ApplicationError::Asset("finalization failed".to_owned()));
            }
            *self.finalized.lock().unwrap() += 1;
            Ok(FinalizeAssetOutcome::Created)
        }
        fn hash(&self, _: &str) -> Result<Sha256, ApplicationError> {
            Ok(Sha256::of_bytes(b"test"))
        }
        fn open(&self, _: &LogicalPath) -> Result<Vec<u8>, ApplicationError> {
            Ok(b"test".to_vec())
        }
        fn verify(&self, _: &StagedAsset) -> Result<bool, ApplicationError> {
            Ok(!self.fail_verify)
        }
        fn discard_stage(&self, _: &StagedAsset) -> Result<(), ApplicationError> {
            *self.discarded.lock().unwrap() += 1;
            Ok(())
        }
        fn quarantine_finalized(
            &self,
            _: &StagedAsset,
            _: &str,
            _: FinalizeAssetOutcome,
        ) -> Result<(), ApplicationError> {
            *self.recovery_markers.lock().unwrap() += 1;
            Ok(())
        }
        fn hash_logical(&self, _: &LogicalPath) -> Result<Sha256, ApplicationError> {
            Ok(Sha256::of_bytes(b"test"))
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
    fn attach_assets_resolves_ordinal_before_staging() {
        let repository = MockRepository::default();
        let assets = MockAssets::default();
        let parent = CaptureService::new(repository.clone(), assets.clone(), FixedClock)
            .capture_text(text("fixture", "existing source text"))
            .unwrap();
        let failing_repository = MockRepository {
            state: repository.state.clone(),
            fail_next_ordinal: true,
            ..Default::default()
        };
        let service = CaptureService::new(failing_repository, assets.clone(), FixedClock);

        let error = service
            .attach_recovered_assets(AttachRecoveredAssetsCommand {
                revision_id: parent.revision_id,
                assets: vec![crate::CaptureImportAsset {
                    path: "source.docx".to_owned(),
                    role: AssetRole::Original,
                }],
                reason: "recover source".to_owned(),
                metadata: Metadata::empty(),
            })
            .unwrap_err();

        assert!(error.to_string().contains("next ordinal failed"));
        assert_eq!(*assets.staged.lock().unwrap(), 0);
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

    #[test]
    fn failed_asset_verification_never_marks_the_revision_ready() {
        let repository = MockRepository::default();
        let assets = MockAssets {
            fail_verify: true,
            ..Default::default()
        };
        let service = CaptureService::new(repository.clone(), assets.clone(), FixedClock);
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
        assert_eq!(state.states[0].1, RawState::Quarantined);
        assert_eq!(*assets.recovery_markers.lock().unwrap(), 1);
    }

    #[test]
    fn failed_ready_transition_preserves_finalized_asset_for_recovery() {
        let repository = MockRepository {
            fail_mark_ready: true,
            ..Default::default()
        };
        let assets = MockAssets::default();
        let service = CaptureService::new(repository.clone(), assets.clone(), FixedClock);
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
        assert_eq!(state.states[0].1, RawState::Quarantined);
        assert_eq!(*assets.recovery_markers.lock().unwrap(), 1);
    }

    #[test]
    fn empty_provider_is_rejected_before_any_write() {
        let repository = MockRepository::default();
        let service = CaptureService::new(repository.clone(), MockAssets::default(), FixedClock);
        assert!(service.capture_text(text(" ", "payload")).is_err());
        assert!(repository.state.lock().unwrap().revisions.is_empty());
    }

    #[test]
    fn mismatched_candidate_route_evidence_is_rejected_before_write() {
        let repository = MockRepository::default();
        let service = CaptureService::new(repository.clone(), MockAssets::default(), FixedClock);
        let payload = "candidate body";
        let result = service.capture_candidate(CandidateCaptureCommand {
            route_evidence: Some(crate::RouteEvidenceCommand {
                route_id: SourceRouteId("source.feishu".to_owned()),
                authorization_id: "fixture-auth".to_owned(),
                source_reference: "https://example.test/chat".to_owned(),
                coverage: RouteCoverage {
                    metadata: true,
                    attachments: false,
                    revisions: true,
                    limitations: Vec::new(),
                },
            }),
            candidate: CandidateEnvelope {
                protocol_version: "1".to_owned(),
                route_id: SourceRouteId("source.kimi".to_owned()),
                source_reference: "https://example.test/chat".to_owned(),
                content_type: ContentType::Document,
                payload_sha256: Sha256::of_bytes(payload.as_bytes()),
                metadata: Metadata::empty(),
                payload: CandidatePayload::Text {
                    text: payload.to_owned(),
                },
                context: None,
                native_id: Some("chat-a".to_owned()),
            },
            assets: Vec::new(),
        });
        assert!(result.is_err());
        assert!(repository.state.lock().unwrap().revisions.is_empty());
    }

    #[test]
    fn browser_bookmark_document_candidate_is_accepted() {
        let repository = MockRepository::default();
        let service = CaptureService::new(repository.clone(), MockAssets::default(), FixedClock);
        let payload = "Example bookmark\nhttps://example.test/bookmark";
        let result = service.capture_candidate(CandidateCaptureCommand {
            route_evidence: None,
            candidate: CandidateEnvelope {
                protocol_version: "1".to_owned(),
                route_id: SourceRouteId("source.browser_bookmarks".to_owned()),
                source_reference: "https://example.test/bookmark".to_owned(),
                content_type: ContentType::Document,
                payload_sha256: Sha256::of_bytes(payload.as_bytes()),
                metadata: Metadata::empty(),
                payload: CandidatePayload::Text {
                    text: payload.to_owned(),
                },
                context: Some("Bookmarks / Test".to_owned()),
                native_id: Some("bookmark-1".to_owned()),
            },
            assets: Vec::new(),
        });
        assert!(result.is_ok());
        assert_eq!(repository.state.lock().unwrap().revisions.len(), 1);
    }

    #[test]
    fn wechat_article_document_candidate_is_accepted() {
        let repository = MockRepository::default();
        let service = CaptureService::new(repository.clone(), MockAssets::default(), FixedClock);
        let payload = "WeChat article body";
        let result = service.capture_candidate(CandidateCaptureCommand {
            route_evidence: None,
            candidate: CandidateEnvelope {
                protocol_version: "1".to_owned(),
                route_id: SourceRouteId("source.wechat_articles".to_owned()),
                source_reference: "https://mp.weixin.qq.com/s/article".to_owned(),
                content_type: ContentType::Document,
                payload_sha256: Sha256::of_bytes(payload.as_bytes()),
                metadata: Metadata::empty(),
                payload: CandidatePayload::Text {
                    text: payload.to_owned(),
                },
                context: Some("WeChat / Favorites".to_owned()),
                native_id: Some("article".to_owned()),
            },
            assets: Vec::new(),
        });
        assert!(result.is_ok());
        assert_eq!(repository.state.lock().unwrap().revisions.len(), 1);
    }

    #[test]
    fn browser_page_document_candidate_is_rejected_before_write() {
        let repository = MockRepository::default();
        let service = CaptureService::new(repository.clone(), MockAssets::default(), FixedClock);
        let payload = "Example page";
        let result = service.capture_candidate(CandidateCaptureCommand {
            route_evidence: None,
            candidate: CandidateEnvelope {
                protocol_version: "1".to_owned(),
                route_id: SourceRouteId("source.browser_pages".to_owned()),
                source_reference: "https://example.test/page".to_owned(),
                content_type: ContentType::Document,
                payload_sha256: Sha256::of_bytes(payload.as_bytes()),
                metadata: Metadata::empty(),
                payload: CandidatePayload::Text {
                    text: payload.to_owned(),
                },
                context: Some("visible page text".to_owned()),
                native_id: None,
            },
            assets: Vec::new(),
        });
        assert!(result.is_err());
        assert!(repository.state.lock().unwrap().revisions.is_empty());
    }
}
