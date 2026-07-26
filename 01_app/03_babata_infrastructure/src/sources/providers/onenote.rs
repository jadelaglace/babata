use std::{
    collections::{BTreeMap, BTreeSet, HashSet},
    fs,
    path::{Path, PathBuf},
};

use babata_application::{
    AcquisitionOutcome, ApplicationError, CaptureImportAsset, DiscoveredCandidate,
    ports::SourceAdapterPort,
};
use babata_domain::{
    AssetRole, CandidateEnvelope, CandidatePayload, CandidateSummary, CapabilityStatus,
    CollectionSessionId, CommonSourceMetadata, ContentType, Metadata, RouteCoverage, Sha256,
    SourceAccessState, SourceHierarchyNode, SourceLimitation, SourceMediaEntry,
    SourceMediaMetadata, SourceRouteDescriptor, SourceRouteId,
};
use lopdf::{Document, Object};
use mailparse::{MailHeaderMap, ParsedMail, parse_mail};
use scraper::{Html, Selector};
use serde::Serialize;

const ROUTE_ID: &str = "source.onenote";
const PAIRED_ADAPTER_VERSION: &str = "onenote-paired-export/1";
const MHT_EXPORT_ADAPTER_VERSION: &str = "onenote-mht-export/1";
const MAX_EXPORT_BYTES: u64 = 512 * 1024 * 1024;
const MAX_MIME_PARTS: usize = 100_000;
const MAX_PDF_PAGES: usize = 100_000;
const MAX_MHT_LIST_FILES: usize = 1_000;
const OVERLAP_NGRAM_CHARS: usize = 12;
const OVERLAP_THRESHOLD_BASIS_POINTS: u16 = 9_000;
const MIN_OVERLAP_TEXT_CHARS: usize = 100;

#[derive(Debug, Clone, Default)]
pub struct OneNoteConfig {
    pub enabled: bool,
}

pub fn descriptor() -> SourceRouteDescriptor {
    SourceRouteDescriptor {
        id: SourceRouteId(ROUTE_ID.to_owned()),
        provider: "onenote".to_owned(),
        status: CapabilityStatus::Enabled,
        activation_phase: "P7".to_owned(),
    }
}

#[derive(Debug, Clone, Default)]
pub struct OneNoteExportAdapter;

impl SourceAdapterPort for OneNoteExportAdapter {
    fn describe(&self) -> SourceRouteDescriptor {
        descriptor()
    }

    fn discover(
        &self,
        session_id: &CollectionSessionId,
        source_reference: &str,
    ) -> Result<Vec<DiscoveredCandidate>, ApplicationError> {
        match parse_source_scope(source_reference)? {
            OneNoteSourceScope::Pair(paths) => {
                let pair = ParsedPair::read(&paths)?;
                let envelope = pair.envelope()?;
                Ok(vec![DiscoveredCandidate {
                    summary: pair.summary(session_id, &envelope),
                    prefetched: Some(envelope),
                }])
            }
            OneNoteSourceScope::MhtList(paths) => {
                ParsedMhtBatch::read(paths)?.candidates(session_id)
            }
        }
    }

    fn collect(
        &self,
        candidate: &CandidateSummary,
        prefetched: Option<&CandidateEnvelope>,
        _requested_attachments: bool,
    ) -> Result<AcquisitionOutcome, ApplicationError> {
        let location = candidate.source_location.as_deref().ok_or_else(|| {
            ApplicationError::Integrity("OneNote candidate has no source location".to_owned())
        })?;
        let expected = prefetched.ok_or_else(|| {
            ApplicationError::Integrity("OneNote candidate envelope is missing".to_owned())
        })?;
        match parse_source_scope(location)? {
            OneNoteSourceScope::Pair(paths) => {
                let pair = ParsedPair::read(&paths)?;
                let current = pair.envelope()?;
                ensure_unchanged(expected, &current)?;
                Ok(AcquisitionOutcome::Found {
                    candidate: Box::new(current),
                    assets: vec![
                        CaptureImportAsset {
                            path: pair.paths.mht.to_string_lossy().into_owned(),
                            role: AssetRole::Export,
                            expected_sha256: Some(pair.mht.sha256.clone()),
                            ..CaptureImportAsset::default()
                        },
                        CaptureImportAsset {
                            path: pair.paths.pdf.to_string_lossy().into_owned(),
                            role: AssetRole::Export,
                            expected_sha256: Some(pair.pdf.sha256.clone()),
                            ..CaptureImportAsset::default()
                        },
                    ],
                })
            }
            OneNoteSourceScope::MhtList(paths) => {
                let batch = ParsedMhtBatch::read(paths)?;
                let export = batch.find_export(expected.native_id.as_deref())?;
                let current = batch.envelope(export)?;
                ensure_unchanged(expected, &current)?;
                Ok(AcquisitionOutcome::Found {
                    candidate: Box::new(current),
                    assets: vec![CaptureImportAsset {
                        path: export.path.to_string_lossy().into_owned(),
                        role: AssetRole::Export,
                        expected_sha256: Some(export.mht.sha256.clone()),
                        ..CaptureImportAsset::default()
                    }],
                })
            }
        }
    }

    fn coverage(&self) -> RouteCoverage {
        RouteCoverage {
            metadata: true,
            attachments: true,
            revisions: false,
            limitations: route_limitations()
                .into_iter()
                .map(|limitation| limitation.detail)
                .collect(),
        }
    }
}

fn ensure_unchanged(
    expected: &CandidateEnvelope,
    current: &CandidateEnvelope,
) -> Result<(), ApplicationError> {
    if expected.route_id != current.route_id
        || expected.native_id != current.native_id
        || expected.payload_sha256 != current.payload_sha256
        || expected.source_reference != current.source_reference
    {
        return Err(ApplicationError::Conflict(
            "OneNote export scope changed after candidate discovery".to_owned(),
        ));
    }
    Ok(())
}

#[derive(Debug)]
enum OneNoteSourceScope {
    Pair(PairPaths),
    MhtList(Vec<PathBuf>),
}

#[derive(Debug, Clone)]
struct PairPaths {
    mht: PathBuf,
    pdf: PathBuf,
}

#[derive(Debug)]
struct ParsedMhtExport {
    path: PathBuf,
    title: String,
    mht: ParsedMht,
}

impl ParsedMhtExport {
    fn read(path: PathBuf) -> Result<Self, ApplicationError> {
        let bytes = read_stable_export(&path)?;
        let parsed = ParsedMht::read(&bytes)?;
        Ok(Self {
            title: export_title(&path),
            path,
            mht: ParsedMht {
                sha256: Sha256::of_bytes(&bytes),
                byte_size: bytes.len() as u64,
                ..parsed
            },
        })
    }

    fn native_id(&self) -> String {
        format!("mht:{}", self.mht.sha256)
    }
}

#[derive(Debug)]
struct ParsedMhtBatch {
    paths: Vec<PathBuf>,
    exports: Vec<ParsedMhtExport>,
    batch_sha256: Sha256,
}

impl ParsedMhtBatch {
    fn read(paths: Vec<PathBuf>) -> Result<Self, ApplicationError> {
        let exports = paths
            .iter()
            .cloned()
            .map(ParsedMhtExport::read)
            .collect::<Result<Vec<_>, _>>()?;
        let identity = exports
            .iter()
            .map(|export| export.mht.sha256.as_str())
            .collect::<Vec<_>>()
            .join(":");
        Ok(Self {
            paths,
            exports,
            batch_sha256: Sha256::of_bytes(identity.as_bytes()),
        })
    }

    fn candidates(
        &self,
        session_id: &CollectionSessionId,
    ) -> Result<Vec<DiscoveredCandidate>, ApplicationError> {
        self.exports
            .iter()
            .map(|export| {
                let envelope = self.envelope(export)?;
                Ok(DiscoveredCandidate {
                    summary: self.summary(session_id, export, &envelope),
                    prefetched: Some(envelope),
                })
            })
            .collect()
    }

    fn find_export(&self, native_id: Option<&str>) -> Result<&ParsedMhtExport, ApplicationError> {
        let native_id = native_id.ok_or_else(|| {
            ApplicationError::Integrity("OneNote MHT candidate native ID is missing".to_owned())
        })?;
        self.exports
            .iter()
            .find(|export| export.native_id() == native_id)
            .ok_or_else(|| {
                ApplicationError::Conflict(
                    "OneNote MHT candidate is no longer present in the explicit list".to_owned(),
                )
            })
    }

    fn overlap_hints(&self, export: &ParsedMhtExport) -> Vec<MhtOverlapHint> {
        self.exports
            .iter()
            .filter(|other| other.mht.sha256 != export.mht.sha256)
            .filter_map(|other| overlap_hint(export, other))
            .collect()
    }

    fn manifest(&self, export: &ParsedMhtExport) -> Result<String, ApplicationError> {
        serde_json::to_string_pretty(&serde_json::json!({
            "schema": "onenote_mht_export_manifest_v1",
            "adapter_version": MHT_EXPORT_ADAPTER_VERSION,
            "identity_sha256": export.mht.sha256.as_str(),
            "identity_scope": "immutable_mht_export",
            "batch_sha256": self.batch_sha256.as_str(),
            "export": {
                "kind": "mht",
                "role": "content_images_and_formatting",
                "sha256": export.mht.sha256.as_str(),
                "byte_size": export.mht.byte_size,
                "root_mime_type": export.mht.root_mime_type,
                "root_html_sha256": export.mht.root_html_sha256.as_str(),
                "root_html_byte_size": export.mht.root_html_byte_size,
                "visible_text_sha256": export.mht.visible_text_sha256.as_str(),
                "normalized_visible_text_chars": export.mht.normalized_text_chars,
                "generator": export.mht.generator,
                "content_types": export.mht.content_types,
                "parts": export.mht.parts,
            },
            "overlap_hints": self.overlap_hints(export),
            "limitations": mht_list_limitations(),
        }))
        .map_err(|error| ApplicationError::Integrity(error.to_string()))
    }

    fn envelope(&self, export: &ParsedMhtExport) -> Result<CandidateEnvelope, ApplicationError> {
        let manifest = self.manifest(export)?;
        let common_metadata = Self::common_metadata(export);
        let hints = self.overlap_hints(export);
        Ok(CandidateEnvelope {
            protocol_version: "1".to_owned(),
            route_id: SourceRouteId(ROUTE_ID.to_owned()),
            source_reference: mht_reference(&export.path),
            content_type: ContentType::Archive,
            payload_sha256: Sha256::of_bytes(manifest.as_bytes()),
            metadata: Metadata::parse(
                &serde_json::json!({
                    "title": export.title,
                    "import_format": "onenote_official_mht_export",
                    "adapter_version": MHT_EXPORT_ADAPTER_VERSION,
                    "identity_sha256": export.mht.sha256.as_str(),
                    "identity_scope": "immutable_mht_export",
                    "batch_sha256": self.batch_sha256.as_str(),
                    "root_html_sha256": export.mht.root_html_sha256.as_str(),
                    "visible_text_sha256": export.mht.visible_text_sha256.as_str(),
                    "normalized_visible_text_chars": export.mht.normalized_text_chars,
                    "overlap_hint_count": hints.len(),
                    "overlap_hints_human_judgment": false,
                    "overlap_hints_confirmed_fact": false,
                    "content_fingerprint": export.mht.sha256.as_str(),
                })
                .to_string(),
            )?,
            payload: CandidatePayload::Text { text: manifest },
            context: common_metadata.context.clone(),
            native_id: Some(export.native_id()),
            common_metadata,
        })
    }

    fn summary(
        &self,
        session_id: &CollectionSessionId,
        export: &ParsedMhtExport,
        envelope: &CandidateEnvelope,
    ) -> CandidateSummary {
        let limitations = mht_list_limitations();
        CandidateSummary {
            candidate_id: format!("onenote_mht_{}", &export.mht.sha256.as_str()[..24]),
            session_id: session_id.clone(),
            route_id: SourceRouteId(ROUTE_ID.to_owned()),
            source_native_id: envelope.native_id.clone(),
            title: Some(export.title.clone()),
            source_location: Some(mht_list_reference(&self.paths)),
            hierarchy: vec![export.title.clone()],
            content_type: ContentType::Archive,
            source_updated_at: None,
            attachment_available: Some(true),
            limitations: limitations
                .iter()
                .map(|limitation| limitation.detail.clone())
                .collect(),
            selection_capabilities: vec![
                "mht_export".to_owned(),
                "batch_export".to_owned(),
                "explicit_export_list".to_owned(),
                "overlap_hint_evidence".to_owned(),
            ],
            common_metadata: envelope.common_metadata.clone(),
        }
    }

    fn common_metadata(export: &ParsedMhtExport) -> CommonSourceMetadata {
        CommonSourceMetadata {
            title: Some(export.title.clone()),
            hierarchy: vec![SourceHierarchyNode {
                kind: Some("official_mht_export_scope".to_owned()),
                name: export.title.clone(),
                native_id: Some(export.mht.sha256.as_str().to_owned()),
                locator: Some(mht_reference(&export.path)),
            }],
            context: Some(format!("Official OneNote MHT export / {}", export.title)),
            limitations: mht_list_limitations(),
            access_state: SourceAccessState::Accessible,
            media: SourceMediaMetadata {
                schema: babata_domain::SOURCE_MEDIA_METADATA_SCHEMA_V1.to_owned(),
                entries: vec![SourceMediaEntry {
                    kind: "content_images_and_formatting_export".to_owned(),
                    media_type: Some(export.mht.root_mime_type.clone()),
                    duration_ms: None,
                    width: None,
                    height: None,
                    page_count: None,
                }],
            },
            ..CommonSourceMetadata::default()
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct MhtOverlapHint {
    kind: String,
    counterpart_native_id: String,
    basis: String,
    this_coverage_basis_points: u16,
    counterpart_coverage_basis_points: u16,
    relative_scope_size: String,
    human_judgment: bool,
    confirmed_fact: bool,
}

fn overlap_hint(export: &ParsedMhtExport, other: &ParsedMhtExport) -> Option<MhtOverlapHint> {
    if export.mht.normalized_text_chars < MIN_OVERLAP_TEXT_CHARS
        || other.mht.normalized_text_chars < MIN_OVERLAP_TEXT_CHARS
    {
        return None;
    }
    let this_coverage = overlap_basis_points(&export.mht.ngrams, &other.mht.ngrams);
    let other_coverage = overlap_basis_points(&other.mht.ngrams, &export.mht.ngrams);
    if this_coverage < OVERLAP_THRESHOLD_BASIS_POINTS
        && other_coverage < OVERLAP_THRESHOLD_BASIS_POINTS
    {
        return None;
    }
    let relative_scope_size = match export
        .mht
        .normalized_text_chars
        .cmp(&other.mht.normalized_text_chars)
    {
        std::cmp::Ordering::Less => "smaller_export",
        std::cmp::Ordering::Equal => "equal_sized_export",
        std::cmp::Ordering::Greater => "larger_export",
    };
    Some(MhtOverlapHint {
        kind: "normalized_text_overlap_candidate".to_owned(),
        counterpart_native_id: other.native_id(),
        basis: format!("normalized_visible_text_fnv1a64_{OVERLAP_NGRAM_CHARS}_char_ngrams"),
        this_coverage_basis_points: this_coverage,
        counterpart_coverage_basis_points: other_coverage,
        relative_scope_size: relative_scope_size.to_owned(),
        human_judgment: false,
        confirmed_fact: false,
    })
}

fn overlap_basis_points(subject: &HashSet<u64>, other: &HashSet<u64>) -> u16 {
    if subject.is_empty() {
        return 0;
    }
    let intersection = subject.intersection(other).count() as u64;
    ((intersection * 10_000) / subject.len() as u64) as u16
}

#[derive(Debug)]
struct ParsedPair {
    paths: PairPaths,
    title: String,
    mht: ParsedMht,
    pdf: ParsedPdf,
    identity_sha256: Sha256,
}

impl ParsedPair {
    fn read(paths: &PairPaths) -> Result<Self, ApplicationError> {
        let mht_bytes = read_stable_export(&paths.mht)?;
        let pdf_bytes = read_stable_export(&paths.pdf)?;
        let mht = ParsedMht::read(&mht_bytes)?;
        let pdf = ParsedPdf::read(&pdf_bytes)?;
        let mht_sha256 = Sha256::of_bytes(&mht_bytes);
        let pdf_sha256 = Sha256::of_bytes(&pdf_bytes);
        let identity_sha256 =
            Sha256::of_bytes(format!("{}:{}", mht_sha256.as_str(), pdf_sha256.as_str()).as_bytes());
        Ok(Self {
            title: export_title(&paths.mht),
            paths: paths.clone(),
            mht: ParsedMht {
                sha256: mht_sha256,
                byte_size: mht_bytes.len() as u64,
                ..mht
            },
            pdf: ParsedPdf {
                sha256: pdf_sha256,
                byte_size: pdf_bytes.len() as u64,
                ..pdf
            },
            identity_sha256,
        })
    }

    fn manifest(&self) -> Result<String, ApplicationError> {
        serde_json::to_string_pretty(&serde_json::json!({
            "schema": "onenote_paired_export_manifest_v1",
            "adapter_version": PAIRED_ADAPTER_VERSION,
            "identity_sha256": self.identity_sha256.as_str(),
            "identity_scope": "immutable_mht_pdf_pair",
            "representations": [
                {
                    "kind": "mht",
                    "role": "content_images_and_formatting",
                    "sha256": self.mht.sha256.as_str(),
                    "byte_size": self.mht.byte_size,
                    "root_html_sha256": self.mht.root_html_sha256.as_str(),
                    "root_html_byte_size": self.mht.root_html_byte_size,
                    "visible_text_sha256": self.mht.visible_text_sha256.as_str(),
                    "normalized_visible_text_chars": self.mht.normalized_text_chars,
                    "generator": self.mht.generator,
                    "content_types": self.mht.content_types,
                    "parts": self.mht.parts,
                },
                {
                    "kind": "pdf",
                    "role": "rendering_and_partition_evidence",
                    "sha256": self.pdf.sha256.as_str(),
                    "byte_size": self.pdf.byte_size,
                    "page_count": self.pdf.page_count,
                    "page_width_points": self.pdf.page_width_points,
                    "page_height_points": self.pdf.page_height_points,
                    "uniform_page_size": self.pdf.uniform_page_size,
                    "creator": self.pdf.creator,
                    "producer": self.pdf.producer,
                    "encrypted": false,
                }
            ],
            "limitations": paired_limitations(),
        }))
        .map_err(|error| ApplicationError::Integrity(error.to_string()))
    }

    fn envelope(&self) -> Result<CandidateEnvelope, ApplicationError> {
        let manifest = self.manifest()?;
        let common_metadata = self.common_metadata();
        Ok(CandidateEnvelope {
            protocol_version: "1".to_owned(),
            route_id: SourceRouteId(ROUTE_ID.to_owned()),
            source_reference: pair_reference(&self.paths),
            content_type: ContentType::Archive,
            payload_sha256: Sha256::of_bytes(manifest.as_bytes()),
            metadata: Metadata::parse(
                &serde_json::json!({
                    "title": self.title,
                    "import_format": "onenote_official_paired_export",
                    "adapter_version": PAIRED_ADAPTER_VERSION,
                    "identity_sha256": self.identity_sha256.as_str(),
                    "identity_scope": "immutable_mht_pdf_pair",
                    "mht_sha256": self.mht.sha256.as_str(),
                    "pdf_sha256": self.pdf.sha256.as_str(),
                    "root_html_sha256": self.mht.root_html_sha256.as_str(),
                    "pdf_page_count": self.pdf.page_count,
                    "representation_roles": {
                        "mht": "content_images_and_formatting",
                        "pdf": "rendering_and_partition_evidence",
                    },
                    "content_fingerprint": self.identity_sha256.as_str(),
                })
                .to_string(),
            )?,
            payload: CandidatePayload::Text { text: manifest },
            context: common_metadata.context.clone(),
            native_id: Some(format!("pair:{}", self.identity_sha256)),
            common_metadata,
        })
    }

    fn summary(
        &self,
        session_id: &CollectionSessionId,
        envelope: &CandidateEnvelope,
    ) -> CandidateSummary {
        let limitations = paired_limitations();
        CandidateSummary {
            candidate_id: format!("onenote_pair_{}", &self.identity_sha256.as_str()[..24]),
            session_id: session_id.clone(),
            route_id: SourceRouteId(ROUTE_ID.to_owned()),
            source_native_id: envelope.native_id.clone(),
            title: Some(self.title.clone()),
            source_location: Some(pair_reference(&self.paths)),
            hierarchy: vec![self.title.clone()],
            content_type: ContentType::Archive,
            source_updated_at: None,
            attachment_available: Some(true),
            limitations: limitations
                .iter()
                .map(|limitation| limitation.detail.clone())
                .collect(),
            selection_capabilities: vec![
                "paired_export".to_owned(),
                "batch_export".to_owned(),
                "explicit_export_scope".to_owned(),
            ],
            common_metadata: envelope.common_metadata.clone(),
        }
    }

    fn common_metadata(&self) -> CommonSourceMetadata {
        CommonSourceMetadata {
            title: Some(self.title.clone()),
            hierarchy: vec![SourceHierarchyNode {
                kind: Some("official_notebook_export".to_owned()),
                name: self.title.clone(),
                native_id: Some(self.identity_sha256.as_str().to_owned()),
                locator: Some(pair_reference(&self.paths)),
            }],
            context: Some(format!(
                "Official OneNote paired PDF/MHT export / {}",
                self.title
            )),
            limitations: paired_limitations(),
            access_state: SourceAccessState::Accessible,
            media: SourceMediaMetadata {
                schema: babata_domain::SOURCE_MEDIA_METADATA_SCHEMA_V1.to_owned(),
                entries: vec![
                    SourceMediaEntry {
                        kind: "content_images_and_formatting_export".to_owned(),
                        media_type: Some("multipart/related".to_owned()),
                        duration_ms: None,
                        width: None,
                        height: None,
                        page_count: None,
                    },
                    SourceMediaEntry {
                        kind: "rendering_and_partition_export".to_owned(),
                        media_type: Some("application/pdf".to_owned()),
                        duration_ms: None,
                        width: None,
                        height: None,
                        page_count: Some(self.pdf.page_count),
                    },
                ],
            },
            ..CommonSourceMetadata::default()
        }
    }
}

#[derive(Debug)]
struct ParsedMht {
    sha256: Sha256,
    byte_size: u64,
    root_mime_type: String,
    root_html_sha256: Sha256,
    root_html_byte_size: u64,
    visible_text_sha256: Sha256,
    normalized_text_chars: usize,
    generator: String,
    ngrams: HashSet<u64>,
    content_types: BTreeMap<String, usize>,
    parts: Vec<MhtPart>,
}

impl ParsedMht {
    fn read(bytes: &[u8]) -> Result<Self, ApplicationError> {
        let parsed = parse_mail(bytes).map_err(|error| {
            ApplicationError::Integrity(format!("invalid OneNote MHT MIME: {error}"))
        })?;
        let root_mime_type = parsed.ctype.mimetype.to_ascii_lowercase();
        if root_mime_type != "multipart/related" && root_mime_type != "text/html" {
            return Err(ApplicationError::Integrity(
                "OneNote MHT root must be multipart/related or text/html".to_owned(),
            ));
        }
        let mut leaves = Vec::new();
        collect_mime_leaves(&parsed, &mut leaves)?;
        if leaves.is_empty() || leaves.len() > MAX_MIME_PARTS {
            return Err(ApplicationError::Integrity(format!(
                "OneNote MHT part count must be between 1 and {MAX_MIME_PARTS}"
            )));
        }
        let mut root_html = None;
        let mut content_types = BTreeMap::new();
        let mut parts = Vec::with_capacity(leaves.len());
        for (index, part) in leaves.into_iter().enumerate() {
            let mime = part.ctype.mimetype.to_ascii_lowercase();
            let body = part.get_body_raw().map_err(|error| {
                ApplicationError::Integrity(format!(
                    "OneNote MHT part {} decoding failed: {error}",
                    index + 1
                ))
            })?;
            let location = part.headers.get_first_value("Content-Location");
            if let Some(location) = &location {
                validate_content_location(location)?;
            }
            *content_types.entry(mime.clone()).or_insert(0) += 1;
            let sha256 = Sha256::of_bytes(&body);
            if mime == "text/html" {
                let html = part.get_body().map_err(|error| {
                    ApplicationError::Integrity(format!(
                        "OneNote MHT HTML decoding failed: {error}"
                    ))
                })?;
                let facts = parse_onenote_html(&html)?;
                if root_html
                    .replace((sha256.clone(), body.len() as u64, facts))
                    .is_some()
                {
                    return Err(ApplicationError::Integrity(
                        "OneNote MHT must contain exactly one HTML root".to_owned(),
                    ));
                }
            }
            parts.push(MhtPart {
                ordinal: index + 1,
                mime,
                byte_size: body.len() as u64,
                sha256,
                content_location: location,
            });
        }
        let (root_html_sha256, root_html_byte_size, html_facts) = root_html.ok_or_else(|| {
            ApplicationError::Integrity("OneNote MHT must contain exactly one HTML root".to_owned())
        })?;
        Ok(Self {
            sha256: Sha256::of_bytes(&[]),
            byte_size: 0,
            root_mime_type,
            root_html_sha256,
            root_html_byte_size,
            visible_text_sha256: Sha256::of_bytes(html_facts.normalized_text.as_bytes()),
            normalized_text_chars: html_facts.normalized_text.chars().count(),
            generator: html_facts.generator,
            ngrams: stable_ngram_hashes(&html_facts.normalized_text),
            content_types,
            parts,
        })
    }
}

#[derive(Debug)]
struct ParsedOneNoteHtml {
    generator: String,
    normalized_text: String,
}

fn parse_onenote_html(html: &str) -> Result<ParsedOneNoteHtml, ApplicationError> {
    let document = Html::parse_document(html);
    let meta_selector = Selector::parse("meta").map_err(|_| {
        ApplicationError::Integrity("OneNote HTML meta selector is invalid".to_owned())
    })?;
    let body_selector = Selector::parse("body").map_err(|_| {
        ApplicationError::Integrity("OneNote HTML body selector is invalid".to_owned())
    })?;
    let mut generator = None;
    let mut program_id = None;
    for element in document.select(&meta_selector) {
        let Some(name) = element.value().attr("name") else {
            continue;
        };
        let content = element.value().attr("content").unwrap_or_default();
        if name.eq_ignore_ascii_case("generator") {
            generator = Some(content.to_owned());
        } else if name.eq_ignore_ascii_case("progid") {
            program_id = Some(content.to_owned());
        }
    }
    let generator =
        generator.filter(|value| value.to_ascii_lowercase().contains("microsoft onenote"));
    if generator.is_none()
        || !program_id.is_some_and(|value| value.eq_ignore_ascii_case("OneNote.File"))
    {
        return Err(ApplicationError::Integrity(
            "MHT HTML metadata does not identify Microsoft OneNote".to_owned(),
        ));
    }
    let normalized_text = document
        .select(&body_selector)
        .flat_map(|body| body.text())
        .flat_map(str::chars)
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    Ok(ParsedOneNoteHtml {
        generator: generator.unwrap_or_default(),
        normalized_text,
    })
}

fn stable_ngram_hashes(value: &str) -> HashSet<u64> {
    let characters = value.chars().collect::<Vec<_>>();
    characters
        .windows(OVERLAP_NGRAM_CHARS)
        .map(|window| {
            window
                .iter()
                .fold(0xcbf2_9ce4_8422_2325, |hash, character| {
                    character
                        .to_string()
                        .as_bytes()
                        .iter()
                        .fold(hash, |next, byte| {
                            (next ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
                        })
                })
        })
        .collect()
}

#[derive(Debug, Clone, Serialize)]
struct MhtPart {
    ordinal: usize,
    mime: String,
    byte_size: u64,
    sha256: Sha256,
    content_location: Option<String>,
}

#[derive(Debug)]
struct ParsedPdf {
    sha256: Sha256,
    byte_size: u64,
    page_count: u32,
    page_width_points: f32,
    page_height_points: f32,
    uniform_page_size: bool,
    creator: Option<String>,
    producer: Option<String>,
}

impl ParsedPdf {
    fn read(bytes: &[u8]) -> Result<Self, ApplicationError> {
        let document = Document::load_mem(bytes).map_err(|error| {
            ApplicationError::Integrity(format!("invalid OneNote PDF: {error}"))
        })?;
        if document.is_encrypted() {
            return Err(ApplicationError::Integrity(
                "encrypted OneNote PDF is not supported".to_owned(),
            ));
        }
        let pages = document.get_pages();
        if pages.is_empty() || pages.len() > MAX_PDF_PAGES {
            return Err(ApplicationError::Integrity(format!(
                "OneNote PDF page count must be between 1 and {MAX_PDF_PAGES}"
            )));
        }
        let mut page_sizes = pages
            .values()
            .map(|page_id| page_size(&document, *page_id))
            .collect::<Result<Vec<_>, _>>()?;
        let (page_width_points, page_height_points) = page_sizes.remove(0);
        let uniform_page_size = page_sizes.iter().all(|(width, height)| {
            (width - page_width_points).abs() < 0.1 && (height - page_height_points).abs() < 0.1
        });
        let creator = pdf_info_string(&document, b"Creator");
        let producer = pdf_info_string(&document, b"Producer");
        if ![creator.as_deref(), producer.as_deref()]
            .into_iter()
            .flatten()
            .any(|value| value.to_ascii_lowercase().contains("onenote"))
        {
            return Err(ApplicationError::Integrity(
                "PDF metadata does not identify Microsoft OneNote".to_owned(),
            ));
        }
        Ok(Self {
            sha256: Sha256::of_bytes(&[]),
            byte_size: 0,
            page_count: pages.len() as u32,
            page_width_points,
            page_height_points,
            uniform_page_size,
            creator,
            producer,
        })
    }
}

fn parse_source_scope(value: &str) -> Result<OneNoteSourceScope, ApplicationError> {
    if value.starts_with("pair:") {
        return parse_pair_reference(value).map(OneNoteSourceScope::Pair);
    }
    if value.starts_with("mht-list:") {
        return parse_mht_list_reference(value).map(OneNoteSourceScope::MhtList);
    }
    Err(ApplicationError::Conflict(
        "OneNote source must be pair:<absolute-mht>|<absolute-pdf> or mht-list:<absolute-mht>|..."
            .to_owned(),
    ))
}

fn parse_pair_reference(value: &str) -> Result<PairPaths, ApplicationError> {
    let value = value.strip_prefix("pair:").ok_or_else(|| {
        ApplicationError::Conflict(
            "OneNote source must be pair:<absolute-mht>|<absolute-pdf>".to_owned(),
        )
    })?;
    let (mht, pdf) = value.split_once('|').ok_or_else(|| {
        ApplicationError::Conflict(
            "OneNote source must contain exactly one MHT/PDF pair".to_owned(),
        )
    })?;
    if mht.trim().is_empty() || pdf.trim().is_empty() || pdf.contains('|') {
        return Err(ApplicationError::Conflict(
            "OneNote source must contain exactly one MHT/PDF pair".to_owned(),
        ));
    }
    let mht = canonical_export_path(Path::new(mht), "mht")?;
    let pdf = canonical_export_path(Path::new(pdf), "pdf")?;
    if mht.parent() != pdf.parent() || mht.file_stem() != pdf.file_stem() {
        return Err(ApplicationError::Conflict(
            "OneNote MHT and PDF must be a same-directory, same-name export pair".to_owned(),
        ));
    }
    Ok(PairPaths { mht, pdf })
}

fn parse_mht_list_reference(value: &str) -> Result<Vec<PathBuf>, ApplicationError> {
    let value = value.strip_prefix("mht-list:").ok_or_else(|| {
        ApplicationError::Conflict(
            "OneNote MHT source must be mht-list:<absolute-mht>|...".to_owned(),
        )
    })?;
    let raw_paths = value.split('|').collect::<Vec<_>>();
    if raw_paths.is_empty()
        || raw_paths.len() > MAX_MHT_LIST_FILES
        || raw_paths.iter().any(|path| path.trim().is_empty())
    {
        return Err(ApplicationError::Conflict(format!(
            "OneNote MHT list must contain between 1 and {MAX_MHT_LIST_FILES} explicit files"
        )));
    }
    let mut paths = raw_paths
        .into_iter()
        .map(|path| canonical_export_path(Path::new(path), "mht"))
        .collect::<Result<Vec<_>, _>>()?;
    let parent = paths[0].parent();
    if paths.iter().any(|path| path.parent() != parent) {
        return Err(ApplicationError::Conflict(
            "OneNote MHT list files must share one explicit directory".to_owned(),
        ));
    }
    paths.sort_by_cached_key(|path| path.to_string_lossy().to_ascii_lowercase());
    let unique = paths.iter().collect::<BTreeSet<_>>();
    if unique.len() != paths.len() {
        return Err(ApplicationError::Conflict(
            "OneNote MHT list must not contain duplicate files".to_owned(),
        ));
    }
    Ok(paths)
}

fn canonical_export_path(path: &Path, extension: &str) -> Result<PathBuf, ApplicationError> {
    if !path.is_absolute() {
        return Err(ApplicationError::Conflict(
            "OneNote export paths must be absolute".to_owned(),
        ));
    }
    if !path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case(extension))
    {
        return Err(ApplicationError::Conflict(format!(
            "OneNote export requires a .{extension} file"
        )));
    }
    let canonical = fs::canonicalize(path).map_err(asset_io)?;
    if !canonical.is_file() {
        return Err(ApplicationError::Conflict(
            "OneNote export paths must identify regular files".to_owned(),
        ));
    }
    Ok(canonical)
}

fn read_stable_export(path: &Path) -> Result<Vec<u8>, ApplicationError> {
    let before = fs::metadata(path).map_err(asset_io)?;
    if before.len() == 0 || before.len() > MAX_EXPORT_BYTES {
        return Err(ApplicationError::Conflict(format!(
            "OneNote export size must be between 1 and {MAX_EXPORT_BYTES} bytes"
        )));
    }
    let bytes = fs::read(path).map_err(asset_io)?;
    let after = fs::metadata(path).map_err(asset_io)?;
    if before.len() != after.len()
        || before.modified().map_err(asset_io)? != after.modified().map_err(asset_io)?
        || bytes.len() as u64 != after.len()
    {
        return Err(ApplicationError::Conflict(
            "OneNote export changed while it was being read".to_owned(),
        ));
    }
    Ok(bytes)
}

fn collect_mime_leaves<'a>(
    part: &'a ParsedMail<'a>,
    leaves: &mut Vec<&'a ParsedMail<'a>>,
) -> Result<(), ApplicationError> {
    if part.subparts.is_empty() {
        leaves.push(part);
    } else {
        for subpart in &part.subparts {
            collect_mime_leaves(subpart, leaves)?;
            if leaves.len() > MAX_MIME_PARTS {
                return Err(ApplicationError::Integrity(format!(
                    "OneNote MHT has more than {MAX_MIME_PARTS} leaf parts"
                )));
            }
        }
    }
    Ok(())
}

fn validate_content_location(value: &str) -> Result<(), ApplicationError> {
    if value.trim().is_empty() {
        return Err(ApplicationError::Integrity(
            "OneNote MHT Content-Location is empty".to_owned(),
        ));
    }
    let normalized = value.to_ascii_lowercase();
    if normalized.contains("../")
        || normalized.contains("..\\")
        || normalized.contains("%2e%2e")
        || normalized.contains('\0')
    {
        return Err(ApplicationError::Integrity(
            "OneNote MHT Content-Location contains path traversal".to_owned(),
        ));
    }
    Ok(())
}

fn page_size(document: &Document, page_id: (u32, u16)) -> Result<(f32, f32), ApplicationError> {
    let mut current = page_id;
    for _ in 0..64 {
        let dictionary = document.get_dictionary(current).map_err(pdf_structure)?;
        if let Ok(media_box) = dictionary.get(b"MediaBox") {
            let media_box = resolve_object(document, media_box)?
                .as_array()
                .map_err(pdf_structure)?;
            if media_box.len() != 4 {
                return Err(ApplicationError::Integrity(
                    "OneNote PDF MediaBox must contain four values".to_owned(),
                ));
            }
            let x0 = resolve_object(document, &media_box[0]).and_then(object_number)?;
            let y0 = resolve_object(document, &media_box[1]).and_then(object_number)?;
            let x1 = resolve_object(document, &media_box[2]).and_then(object_number)?;
            let y1 = resolve_object(document, &media_box[3]).and_then(object_number)?;
            let width = (x1 - x0).abs();
            let height = (y1 - y0).abs();
            if width <= 0.0 || height <= 0.0 {
                return Err(ApplicationError::Integrity(
                    "OneNote PDF page size is invalid".to_owned(),
                ));
            }
            return Ok((width, height));
        }
        current = dictionary
            .get(b"Parent")
            .and_then(Object::as_reference)
            .map_err(pdf_structure)?;
    }
    Err(ApplicationError::Integrity(
        "OneNote PDF page tree exceeded the inheritance limit".to_owned(),
    ))
}

fn resolve_object<'a>(
    document: &'a Document,
    object: &'a Object,
) -> Result<&'a Object, ApplicationError> {
    match object {
        Object::Reference(id) => document.get_object(*id).map_err(pdf_structure),
        _ => Ok(object),
    }
}

fn object_number(object: &Object) -> Result<f32, ApplicationError> {
    object.as_float().map_err(pdf_structure)
}

fn pdf_info_string(document: &Document, key: &[u8]) -> Option<String> {
    let info = document.trailer.get(b"Info").ok()?;
    let dictionary = match info {
        Object::Reference(id) => document.get_dictionary(*id).ok()?,
        Object::Dictionary(dictionary) => dictionary,
        _ => return None,
    };
    let value = resolve_object(document, dictionary.get(key).ok()?).ok()?;
    match value {
        Object::String(bytes, _) => Some(decode_pdf_string(bytes)),
        _ => None,
    }
}

fn decode_pdf_string(bytes: &[u8]) -> String {
    if let Some(body) = bytes.strip_prefix(&[0xfe, 0xff]) {
        let units = body
            .chunks_exact(2)
            .map(|pair| u16::from_be_bytes([pair[0], pair[1]]))
            .collect::<Vec<_>>();
        String::from_utf16_lossy(&units)
    } else if let Some(body) = bytes.strip_prefix(&[0xff, 0xfe]) {
        let units = body
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect::<Vec<_>>();
        String::from_utf16_lossy(&units)
    } else {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

fn pair_reference(paths: &PairPaths) -> String {
    format!(
        "pair:{}|{}",
        paths.mht.to_string_lossy(),
        paths.pdf.to_string_lossy()
    )
}

fn mht_reference(path: &Path) -> String {
    format!("mht:{}", path.to_string_lossy())
}

fn mht_list_reference(paths: &[PathBuf]) -> String {
    format!(
        "mht-list:{}",
        paths
            .iter()
            .map(|path| path.to_string_lossy())
            .collect::<Vec<_>>()
            .join("|")
    )
}

fn export_title(path: &Path) -> String {
    path.file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("OneNote export")
        .to_owned()
}

fn paired_limitations() -> Vec<SourceLimitation> {
    vec![
        SourceLimitation {
            code: "complementary_export_representations".to_owned(),
            detail: "PDF preserves rendering and partition evidence while MHT preserves content, images, and formatting; neither replaces the other".to_owned(),
        },
        SourceLimitation {
            code: "native_page_identity_unavailable".to_owned(),
            detail: "PDF page numbers are export layout evidence, not native OneNote page or section IDs; page-level splitting belongs to C1".to_owned(),
        },
        SourceLimitation {
            code: "export_scoped_identity".to_owned(),
            detail: "identity is scoped to the immutable MHT/PDF hash pair; cross-export notebook matching is unavailable".to_owned(),
        },
    ]
}

fn mht_list_limitations() -> Vec<SourceLimitation> {
    vec![
        SourceLimitation {
            code: "mht_only_representation".to_owned(),
            detail: "this explicit export has MHT content, images, and formatting but no paired PDF rendering evidence".to_owned(),
        },
        SourceLimitation {
            code: "overlap_is_not_hierarchy".to_owned(),
            detail: "batch-local normalized-text overlap is machine evidence only; it does not confirm a native OneNote parent, child, page, or section relation".to_owned(),
        },
        SourceLimitation {
            code: "c1_segmentation_pending".to_owned(),
            detail: "semantic segmentation, deduplication, and hierarchy confirmation remain later traceable C1 work".to_owned(),
        },
        SourceLimitation {
            code: "export_scoped_identity".to_owned(),
            detail: "identity is scoped to the immutable MHT hash; cross-export native OneNote identity is unavailable".to_owned(),
        },
    ]
}

fn route_limitations() -> Vec<SourceLimitation> {
    let mut limitations = paired_limitations();
    limitations.extend(mht_list_limitations());
    limitations
}

fn pdf_structure(error: lopdf::Error) -> ApplicationError {
    ApplicationError::Integrity(format!("invalid OneNote PDF structure: {error}"))
}

fn asset_io(error: std::io::Error) -> ApplicationError {
    ApplicationError::Asset(format!("OneNote filesystem {:?} failure", error.kind()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use babata_application::{
        CollectorSessionService, StartCollectionCommand, ports::RawRepositoryPort,
    };
    use babata_domain::{CollectionItemState, CollectionSelection, RecollectionState};
    use lopdf::{Dictionary, Stream, StringFormat, dictionary};
    use std::fmt::Write as _;
    use tempfile::tempdir;

    use crate::{
        FileAssetStore, SystemClock,
        paths::{DataPaths, ensure_layout},
    };

    #[test]
    fn paired_export_discovers_one_complementary_archive_candidate() {
        let temporary = tempdir().unwrap();
        let paths = write_fixture_pair(temporary.path(), "notebook");
        let adapter = OneNoteExportAdapter;
        let candidates = adapter
            .discover(&CollectionSessionId::new(), &pair_reference(&paths))
            .unwrap();
        assert_eq!(candidates.len(), 1);
        let candidate = &candidates[0];
        assert_eq!(candidate.summary.content_type, ContentType::Archive);
        assert_eq!(candidate.summary.title.as_deref(), Some("notebook"));
        let manifest = match &candidate.prefetched.as_ref().unwrap().payload {
            CandidatePayload::Text { text } => text,
        };
        assert!(manifest.contains("\"adapter_version\": \"onenote-paired-export/1\""));
        assert!(manifest.contains("content_images_and_formatting"));
        assert!(manifest.contains("rendering_and_partition_evidence"));
        assert!(manifest.contains("\"page_count\": 1"));
        let acquisition = adapter
            .collect(&candidate.summary, candidate.prefetched.as_ref(), true)
            .unwrap();
        let AcquisitionOutcome::Found { assets, .. } = acquisition else {
            panic!("paired export should be collectable");
        };
        assert_eq!(assets.len(), 2);
        for asset in assets {
            assert_eq!(
                asset.expected_sha256,
                Some(Sha256::of_bytes(&fs::read(asset.path).unwrap()))
            );
        }
    }

    #[test]
    fn explicit_mht_list_discovers_exports_and_nonfactual_overlap_hints() {
        let temporary = tempdir().unwrap();
        let child_text = "独立内容".repeat(40);
        let parent = temporary.path().join("parent.mht");
        let child = temporary.path().join("child.mht");
        fs::write(
            &parent,
            fixture_single_mht(&format!("{child_text}父范围补充"), true),
        )
        .unwrap();
        fs::write(&child, fixture_single_mht(&child_text, true)).unwrap();
        let paths = canonical_mht_paths(&[parent, child]);
        let reference = mht_list_reference(&paths);
        let adapter = OneNoteExportAdapter;

        let candidates = adapter
            .discover(&CollectionSessionId::new(), &reference)
            .unwrap();
        assert_eq!(candidates.len(), 2);
        assert!(
            candidates
                .iter()
                .all(|candidate| candidate.summary.content_type == ContentType::Archive)
        );
        let child = candidates
            .iter()
            .find(|candidate| candidate.summary.title.as_deref() == Some("child"))
            .unwrap();
        let manifest = match &child.prefetched.as_ref().unwrap().payload {
            CandidatePayload::Text { text } => serde_json::from_str::<serde_json::Value>(text)
                .expect("MHT manifest should be JSON"),
        };
        let hints = manifest["overlap_hints"].as_array().unwrap();
        assert_eq!(manifest["adapter_version"], "onenote-mht-export/1");
        assert_eq!(hints.len(), 1);
        assert_eq!(hints[0]["kind"], "normalized_text_overlap_candidate");
        assert_eq!(hints[0]["relative_scope_size"], "smaller_export");
        assert_eq!(hints[0]["human_judgment"], false);
        assert_eq!(hints[0]["confirmed_fact"], false);
        assert_eq!(hints[0]["this_coverage_basis_points"], 10_000);

        let acquisition = adapter
            .collect(&child.summary, child.prefetched.as_ref(), true)
            .unwrap();
        let AcquisitionOutcome::Found { assets, .. } = acquisition else {
            panic!("MHT export should be collectable");
        };
        assert_eq!(assets.len(), 1);
        assert_eq!(
            assets[0].expected_sha256,
            Some(Sha256::of_bytes(&fs::read(&assets[0].path).unwrap()))
        );
    }

    #[test]
    fn mht_list_scope_generator_and_mime_fail_closed() {
        let temporary = tempdir().unwrap();
        let multipart = temporary.path().join("multipart.mht");
        let single = temporary.path().join("single.mht");
        fs::write(&multipart, fixture_mht(true, false, false)).unwrap();
        fs::write(&single, fixture_single_mht("单体 OneNote 正文", true)).unwrap();
        let paths = canonical_mht_paths(&[multipart.clone(), single.clone()]);
        let adapter = OneNoteExportAdapter;
        assert_eq!(
            adapter
                .discover(&CollectionSessionId::new(), &mht_list_reference(&paths))
                .unwrap()
                .len(),
            2
        );
        assert!(
            adapter
                .discover(&CollectionSessionId::new(), "mht-list:relative.mht")
                .is_err()
        );
        assert!(
            adapter
                .discover(
                    &CollectionSessionId::new(),
                    &format!("mht-list:{}|{}", paths[0].display(), paths[0].display())
                )
                .is_err()
        );

        let other_root = temporary.path().join("other");
        fs::create_dir(&other_root).unwrap();
        let other = other_root.join("other.mht");
        fs::write(&other, fixture_single_mht("另一个范围", true)).unwrap();
        assert!(
            adapter
                .discover(
                    &CollectionSessionId::new(),
                    &format!(
                        "mht-list:{}|{}",
                        paths[0].display(),
                        other.canonicalize().unwrap().display()
                    )
                )
                .is_err()
        );

        fs::write(&single, fixture_single_mht("不是 OneNote", false)).unwrap();
        assert!(
            adapter
                .discover(
                    &CollectionSessionId::new(),
                    &mht_list_reference(&canonical_mht_paths(&[single]))
                )
                .is_err()
        );
    }

    #[test]
    fn mht_batch_change_after_discovery_is_rejected_without_a_cache() {
        let temporary = tempdir().unwrap();
        let first = temporary.path().join("first.mht");
        let second = temporary.path().join("second.mht");
        fs::write(&first, fixture_single_mht(&"共同内容".repeat(40), true)).unwrap();
        fs::write(
            &second,
            fixture_single_mht(&format!("{}补充", "共同内容".repeat(40)), true),
        )
        .unwrap();
        let paths = canonical_mht_paths(&[first, second.clone()]);
        let adapter = OneNoteExportAdapter;
        let candidate = adapter
            .discover(&CollectionSessionId::new(), &mht_list_reference(&paths))
            .unwrap()
            .remove(0);
        fs::write(&second, fixture_single_mht("已变化的批内内容", true)).unwrap();

        let error = adapter
            .collect(&candidate.summary, candidate.prefetched.as_ref(), true)
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("changed after candidate discovery")
        );
    }

    #[test]
    fn scope_and_mime_structure_fail_closed() {
        let temporary = tempdir().unwrap();
        let paths = write_fixture_pair(temporary.path(), "notebook");
        let adapter = OneNoteExportAdapter;
        assert!(
            adapter
                .discover(
                    &CollectionSessionId::new(),
                    "pair:relative.mht|relative.pdf"
                )
                .is_err()
        );

        let other_pdf = temporary.path().join("other.pdf");
        fs::write(&other_pdf, fixture_pdf(false)).unwrap();
        assert!(
            adapter
                .discover(
                    &CollectionSessionId::new(),
                    &format!(
                        "pair:{}|{}",
                        paths.mht.canonicalize().unwrap().display(),
                        other_pdf.canonicalize().unwrap().display()
                    )
                )
                .is_err()
        );

        fs::write(&paths.mht, fixture_mht(false, false, false)).unwrap();
        assert!(
            adapter
                .discover(&CollectionSessionId::new(), &pair_reference(&paths))
                .is_err()
        );
        fs::write(&paths.mht, fixture_mht(true, true, false)).unwrap();
        assert!(
            adapter
                .discover(&CollectionSessionId::new(), &pair_reference(&paths))
                .is_err()
        );
        fs::write(&paths.mht, fixture_mht(true, false, true)).unwrap();
        assert!(
            adapter
                .discover(&CollectionSessionId::new(), &pair_reference(&paths))
                .is_err()
        );
    }

    #[test]
    fn malformed_and_encrypted_pdfs_are_rejected() {
        let temporary = tempdir().unwrap();
        let paths = write_fixture_pair(temporary.path(), "notebook");
        let adapter = OneNoteExportAdapter;
        fs::write(&paths.pdf, b"%PDF-1.7 truncated").unwrap();
        assert!(
            adapter
                .discover(&CollectionSessionId::new(), &pair_reference(&paths))
                .is_err()
        );
        fs::write(&paths.pdf, fixture_pdf(true)).unwrap();
        assert!(
            adapter
                .discover(&CollectionSessionId::new(), &pair_reference(&paths))
                .is_err()
        );
    }

    #[test]
    fn pair_change_after_discovery_is_rejected_without_a_cache() {
        let temporary = tempdir().unwrap();
        let paths = write_fixture_pair(temporary.path(), "notebook");
        let adapter = OneNoteExportAdapter;
        let candidate = adapter
            .discover(&CollectionSessionId::new(), &pair_reference(&paths))
            .unwrap()
            .remove(0);
        let changed = String::from_utf8(fixture_mht(true, false, false))
            .unwrap()
            .replace("<body>fixture</body>", "<body>changed</body>")
            .into_bytes();
        fs::write(&paths.mht, changed).unwrap();
        let error = adapter
            .collect(&candidate.summary, candidate.prefetched.as_ref(), true)
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("changed after candidate discovery")
        );
    }

    #[test]
    fn collector_persists_both_exports_and_recollects_unchanged() {
        let temporary = tempdir().unwrap();
        let export_paths = write_fixture_pair(temporary.path(), "notebook");
        let data_paths = DataPaths::new(temporary.path().join("data"));
        ensure_layout(&data_paths).unwrap();
        let repository = crate::open_collection_database(&data_paths, 100).unwrap();
        let service = CollectorSessionService::new(
            repository.clone(),
            FileAssetStore::new(data_paths),
            SystemClock,
            vec![Box::new(OneNoteExportAdapter)],
        );
        let session = service
            .start(StartCollectionCommand {
                route_id: SourceRouteId(ROUTE_ID.to_owned()),
                source_reference: pair_reference(&export_paths),
                scope_description: "one synthetic paired export".to_owned(),
                authorisation_id: "fixture-authorisation".to_owned(),
            })
            .unwrap();
        let candidates = service.candidates(&session.session_id).unwrap();
        assert_eq!(candidates.len(), 1);
        let items = service
            .select(CollectionSelection {
                session_id: session.session_id.clone(),
                candidate_ids: vec![candidates[0].candidate_id.clone()],
                scope_description: "the paired fixture export".to_owned(),
                confirmed: true,
                authorised_context: "fixture-authorisation".to_owned(),
                requested_attachments: true,
            })
            .unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].state, CollectionItemState::Saved);
        let detail = repository
            .load_detail(items[0].item_id.as_ref().unwrap())
            .unwrap();
        assert_eq!(detail.revisions.len(), 1);
        assert_eq!(detail.assets.len(), 2);
        assert!(
            detail
                .assets
                .iter()
                .all(|asset| asset.role == AssetRole::Export)
        );

        let recollected = service.recollect_session(&session.session_id).unwrap();
        assert_eq!(recollected.len(), 1);
        assert_eq!(recollected[0].state, RecollectionState::Unchanged);
        assert!(recollected[0].new_revision_id.is_none());
        assert_eq!(
            repository
                .load_detail(items[0].item_id.as_ref().unwrap())
                .unwrap()
                .revisions
                .len(),
            1
        );
    }

    #[test]
    fn collector_persists_explicit_mht_exports_and_recollects_unchanged() {
        let temporary = tempdir().unwrap();
        let first = temporary.path().join("first.mht");
        let second = temporary.path().join("second.mht");
        fs::write(&first, fixture_single_mht("第一份明确导出", true)).unwrap();
        fs::write(&second, fixture_single_mht("第二份明确导出", true)).unwrap();
        let export_paths = canonical_mht_paths(&[first, second]);
        let data_paths = DataPaths::new(temporary.path().join("data"));
        ensure_layout(&data_paths).unwrap();
        let repository = crate::open_collection_database(&data_paths, 100).unwrap();
        let service = CollectorSessionService::new(
            repository.clone(),
            FileAssetStore::new(data_paths),
            SystemClock,
            vec![Box::new(OneNoteExportAdapter)],
        );
        let session = service
            .start(StartCollectionCommand {
                route_id: SourceRouteId(ROUTE_ID.to_owned()),
                source_reference: mht_list_reference(&export_paths),
                scope_description: "two explicit synthetic MHT exports".to_owned(),
                authorisation_id: "fixture-authorisation".to_owned(),
            })
            .unwrap();
        let candidates = service.candidates(&session.session_id).unwrap();
        assert_eq!(candidates.len(), 2);
        let items = service
            .select(CollectionSelection {
                session_id: session.session_id.clone(),
                candidate_ids: candidates
                    .iter()
                    .map(|candidate| candidate.candidate_id.clone())
                    .collect(),
                scope_description: "both explicit fixture exports".to_owned(),
                confirmed: true,
                authorised_context: "fixture-authorisation".to_owned(),
                requested_attachments: true,
            })
            .unwrap();
        assert_eq!(items.len(), 2);
        assert!(
            items
                .iter()
                .all(|item| item.state == CollectionItemState::Saved)
        );
        for item in &items {
            let detail = repository
                .load_detail(item.item_id.as_ref().unwrap())
                .unwrap();
            assert_eq!(detail.revisions.len(), 1);
            assert_eq!(detail.assets.len(), 1);
            assert_eq!(detail.assets[0].role, AssetRole::Export);
        }

        let recollected = service.recollect_session(&session.session_id).unwrap();
        assert_eq!(recollected.len(), 2);
        assert!(
            recollected
                .iter()
                .all(|item| item.state == RecollectionState::Unchanged)
        );
        assert!(
            recollected
                .iter()
                .all(|item| item.new_revision_id.is_none())
        );
    }

    fn write_fixture_pair(root: &Path, stem: &str) -> PairPaths {
        let mht = root.join(format!("{stem}.mht"));
        let pdf = root.join(format!("{stem}.pdf"));
        fs::write(&mht, fixture_mht(true, false, false)).unwrap();
        fs::write(&pdf, fixture_pdf(false)).unwrap();
        PairPaths {
            mht: mht.canonicalize().unwrap(),
            pdf: pdf.canonicalize().unwrap(),
        }
    }

    fn fixture_mht(include_html: bool, duplicate_html: bool, traversal: bool) -> Vec<u8> {
        let html_location = if traversal {
            "file:///C:/fixture/../secret.htm"
        } else {
            "file:///C:/fixture/root.htm"
        };
        let mut parts = String::new();
        if include_html {
            write!(
                parts,
                "--fixture\r\nContent-Type: text/html; charset=utf-8\r\nContent-Location: {html_location}\r\n\r\n{}\r\n",
                fixture_html("fixture", true)
            )
            .unwrap();
        }
        if duplicate_html {
            write!(
                parts,
                "--fixture\r\nContent-Type: text/html; charset=utf-8\r\nContent-Location: file:///C:/fixture/second.htm\r\n\r\n{}\r\n",
                fixture_html("second", true)
            )
            .unwrap();
        }
        parts.push_str(
            "--fixture\r\nContent-Type: image/png\r\nContent-Transfer-Encoding: base64\r\nContent-Location: file:///C:/fixture/image.png\r\n\r\nZml4dHVyZQ==\r\n--fixture--\r\n",
        );
        format!(
            "MIME-Version: 1.0\r\nContent-Type: multipart/related; boundary=\"fixture\"\r\n\r\n{parts}"
        )
        .into_bytes()
    }

    fn fixture_single_mht(body: &str, onenote: bool) -> Vec<u8> {
        format!(
            "MIME-Version: 1.0\r\nContent-Location: file:///C:/fixture/root.htm\r\nContent-Type: text/html; charset=utf-8\r\n\r\n{}",
            fixture_html(body, onenote)
        )
        .into_bytes()
    }

    fn fixture_html(body: &str, onenote: bool) -> String {
        let metadata = if onenote {
            r#"<meta name="ProgId" content="OneNote.File"><meta name="Generator" content="Microsoft OneNote 15">"#
        } else {
            r#"<meta name="Generator" content="Browser Export">"#
        };
        format!("<html><head>{metadata}</head><body>{body}</body></html>")
    }

    fn canonical_mht_paths(paths: &[PathBuf]) -> Vec<PathBuf> {
        let mut paths = paths
            .iter()
            .map(|path| path.canonicalize().unwrap())
            .collect::<Vec<_>>();
        paths.sort_by_cached_key(|path| path.to_string_lossy().to_ascii_lowercase());
        paths
    }

    fn fixture_pdf(encrypted: bool) -> Vec<u8> {
        let mut document = Document::with_version("1.7");
        let pages_id = document.new_object_id();
        let content_id = document.add_object(Stream::new(dictionary! {}, Vec::new()));
        let page_object_id = document.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Contents" => content_id,
        });
        document.objects.insert(
            pages_id,
            Object::Dictionary(dictionary! {
                "Type" => "Pages",
                "Kids" => vec![page_object_id.into()],
                "Count" => 1,
                "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
            }),
        );
        let catalog_id = document.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        let creator = "Microsoft® OneNote® 2021";
        let mut creator_utf16 = vec![0xfe, 0xff];
        creator_utf16.extend(creator.encode_utf16().flat_map(u16::to_be_bytes));
        let info_id = document.add_object(dictionary! {
            "Creator" => Object::String(creator_utf16.clone(), StringFormat::Hexadecimal),
            "Producer" => Object::String(creator_utf16, StringFormat::Hexadecimal),
        });
        document.trailer.set("Root", catalog_id);
        document.trailer.set("Info", info_id);
        if encrypted {
            let encryption_id = document.add_object(Dictionary::new());
            document.trailer.set("Encrypt", encryption_id);
        }
        let mut bytes = Vec::new();
        document.save_to(&mut bytes).unwrap();
        bytes
    }
}
