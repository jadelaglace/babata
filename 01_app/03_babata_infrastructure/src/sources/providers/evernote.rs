use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::SystemTime,
};

use aes::{
    Aes128,
    cipher::{BlockDecryptMut, KeyIvInit, block_padding::Pkcs7},
};
use babata_application::{
    AcquisitionOutcome, ApplicationError, CaptureImportAsset, DiscoveredCandidate,
    ports::SourceAdapterPort,
};
use babata_domain::{
    AssetRole, CandidateEnvelope, CandidatePayload, CandidateSummary, CapabilityStatus,
    CollectionSessionId, CommonSourceMetadata, ContentType, Metadata, RouteCoverage, Sha256,
    SourceAccessState, SourceHierarchyNode, SourceLimitation, SourceMediaEntry,
    SourceMediaMetadata, SourceRouteDescriptor, SourceRouteId, UtcTimestamp,
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256 as Sha256Digest;

const ROUTE_ID: &str = "source.evernote";
const ADAPTER_VERSION: &str = "yinxiang-notes/1";
const HMAC_SEED: &[u8] = b"{22C58AC3-F1C7-4D96-8B88-5E4BBF505817}";
const MAX_EXPORT_BYTES: u64 = 512 * 1024 * 1024;
const MAX_NOTES: usize = 100_000;
const ENC0_OVERHEAD_BYTES: usize = 4 + 16 + 16 + 16 + 32;

type HmacSha256 = Hmac<Sha256Digest>;
type Aes128CbcDecryptor = cbc::Decryptor<Aes128>;

#[derive(Debug, Clone, Default)]
pub struct EvernoteConfig {
    pub enabled: bool,
}

pub fn descriptor() -> SourceRouteDescriptor {
    SourceRouteDescriptor {
        id: SourceRouteId(ROUTE_ID.to_owned()),
        provider: "evernote".to_owned(),
        status: CapabilityStatus::Enabled,
        activation_phase: "P7".to_owned(),
    }
}

#[derive(Debug)]
pub struct EvernoteNotesAdapter {
    runtime_root: PathBuf,
    cache: Mutex<HashMap<PathBuf, CachedExport>>,
}

#[derive(Debug, Clone)]
struct CachedExport {
    source_stamp: SourceStamp,
    export: Arc<ParsedExport>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct SourceStamp {
    byte_size: u64,
    modified: SystemTime,
}

impl EvernoteNotesAdapter {
    pub fn new(runtime_root: PathBuf) -> Self {
        Self {
            runtime_root,
            cache: Mutex::new(HashMap::new()),
        }
    }

    fn load(&self, path: &Path) -> Result<Arc<ParsedExport>, ApplicationError> {
        let canonical = canonical_notes_path(path)?;
        let source_stamp = source_stamp(&canonical)?;
        if let Some(export) = self
            .cache
            .lock()
            .map_err(|_| ApplicationError::Storage("Evernote cache lock poisoned".to_owned()))?
            .get(&canonical)
            .filter(|cached| cached.source_stamp == source_stamp)
            .map(|cached| cached.export.clone())
        {
            return Ok(export);
        }
        let mut parsed = ParsedExport::read(&canonical)?;
        parsed.decrypted_enex_path = write_decrypted_enex(&self.runtime_root, &parsed)?;
        let source_stamp = parsed.source_stamp.clone();
        let parsed = Arc::new(parsed);
        self.cache
            .lock()
            .map_err(|_| ApplicationError::Storage("Evernote cache lock poisoned".to_owned()))?
            .insert(
                canonical,
                CachedExport {
                    source_stamp,
                    export: parsed.clone(),
                },
            );
        Ok(parsed)
    }
}

impl SourceAdapterPort for EvernoteNotesAdapter {
    fn describe(&self) -> SourceRouteDescriptor {
        descriptor()
    }

    fn discover(
        &self,
        session_id: &CollectionSessionId,
        source_reference: &str,
    ) -> Result<Vec<DiscoveredCandidate>, ApplicationError> {
        let path = source_reference.strip_prefix("notes:").ok_or_else(|| {
            ApplicationError::Conflict(
                "Evernote source must be notes:<absolute-path-to-one-.notes-export>".to_owned(),
            )
        })?;
        if path.trim().is_empty() {
            return Err(ApplicationError::Conflict(
                "Evernote source path is empty".to_owned(),
            ));
        }
        let export = self.load(Path::new(path))?;
        export.discovered_candidates(session_id)
    }

    fn collect(
        &self,
        candidate: &CandidateSummary,
        prefetched: Option<&CandidateEnvelope>,
        requested_attachments: bool,
    ) -> Result<AcquisitionOutcome, ApplicationError> {
        let source_location = candidate.source_location.as_deref().ok_or_else(|| {
            ApplicationError::Integrity("Evernote candidate has no source location".to_owned())
        })?;
        let (path, ordinal) = parse_candidate_location(source_location)?;
        let export = self.load(&path)?;
        let expected = prefetched.ok_or_else(|| {
            ApplicationError::Integrity("Evernote candidate envelope is missing".to_owned())
        })?;
        let (current, assets) = if let Some(ordinal) = ordinal {
            let current = export.note_envelope(ordinal)?;
            let assets = if requested_attachments {
                export.materialize_note_resources(&self.runtime_root, ordinal)?
            } else {
                Vec::new()
            };
            (current, assets)
        } else {
            (
                export.export_envelope()?,
                vec![
                    CaptureImportAsset {
                        path: export.path.to_string_lossy().into_owned(),
                        role: AssetRole::Original,
                        expected_sha256: None,
                    },
                    CaptureImportAsset {
                        path: export.decrypted_enex_path.to_string_lossy().into_owned(),
                        role: AssetRole::Export,
                        expected_sha256: None,
                    },
                ],
            )
        };
        if expected.route_id != current.route_id
            || expected.native_id != current.native_id
            || expected.payload_sha256 != current.payload_sha256
        {
            return Err(ApplicationError::Conflict(
                "Evernote export changed after candidate discovery".to_owned(),
            ));
        }
        Ok(AcquisitionOutcome::Found {
            candidate: Box::new(current),
            assets,
        })
    }

    fn coverage(&self) -> RouteCoverage {
        RouteCoverage {
            metadata: true,
            attachments: true,
            revisions: false,
            limitations: vec![
                "identity is scoped to one immutable .notes export because this format has no note GUID or update timestamp".to_owned(),
                "resources are imported only when attachments are explicitly requested".to_owned(),
                "notebook hierarchy and cross-export note matching are unavailable in this export format".to_owned(),
            ],
        }
    }
}

#[derive(Debug)]
struct ParsedExport {
    path: PathBuf,
    source_stamp: SourceStamp,
    byte_size: u64,
    sha256: Sha256,
    document: EnExport,
    notes: Vec<ParsedNote>,
    resource_count: usize,
    decrypted_enex_path: PathBuf,
}

impl ParsedExport {
    fn read(path: &Path) -> Result<Self, ApplicationError> {
        let metadata = fs::metadata(path).map_err(asset_io)?;
        if !metadata.is_file() {
            return Err(ApplicationError::Conflict(
                "Evernote source must be one regular .notes file".to_owned(),
            ));
        }
        if metadata.len() == 0 || metadata.len() > MAX_EXPORT_BYTES {
            return Err(ApplicationError::Conflict(format!(
                "Evernote export size must be between 1 and {MAX_EXPORT_BYTES} bytes"
            )));
        }
        let bytes = fs::read(path).map_err(asset_io)?;
        let source_stamp = source_stamp(path)?;
        if metadata.len() != source_stamp.byte_size
            || metadata.modified().map_err(asset_io)? != source_stamp.modified
        {
            return Err(ApplicationError::Conflict(
                "Evernote export changed while it was being read".to_owned(),
            ));
        }
        reject_entity_declarations(&bytes)?;
        let sha256 = Sha256::of_bytes(&bytes);
        let mut document: EnExport =
            quick_xml::de::from_reader(bytes.as_slice()).map_err(|error| {
                ApplicationError::Integrity(format!("invalid Evernote .notes XML: {error}"))
            })?;
        if document.notes.is_empty() || document.notes.len() > MAX_NOTES {
            return Err(ApplicationError::Integrity(format!(
                "Evernote export note count must be between 1 and {MAX_NOTES}"
            )));
        }
        let export_sha = sha256.as_str().to_owned();
        let workers = std::thread::available_parallelism()
            .map_or(1, std::num::NonZeroUsize::get)
            .min(document.notes.len());
        let chunk_size = document.notes.len().div_ceil(workers);
        let grouped = std::thread::scope(|scope| {
            let handles = document
                .notes
                .chunks_mut(chunk_size)
                .enumerate()
                .map(|(chunk_index, chunk)| {
                    let export_sha = export_sha.clone();
                    scope.spawn(move || {
                        chunk
                            .iter_mut()
                            .enumerate()
                            .map(|(offset, note)| {
                                parse_note(note, chunk_index * chunk_size + offset + 1, &export_sha)
                            })
                            .collect::<Result<Vec<_>, _>>()
                    })
                })
                .collect::<Vec<_>>();
            handles
                .into_iter()
                .map(|handle| {
                    handle.join().map_err(|_| {
                        ApplicationError::Integrity(
                            "Evernote note decryption worker panicked".to_owned(),
                        )
                    })?
                })
                .collect::<Result<Vec<_>, _>>()
        })?;
        let notes = grouped.into_iter().flatten().collect::<Vec<_>>();
        let resource_count = notes.iter().try_fold(0usize, |count, note| {
            count.checked_add(note.resources.len()).ok_or_else(|| {
                ApplicationError::Integrity("Evernote resource count overflow".to_owned())
            })
        })?;
        Ok(Self {
            path: path.to_path_buf(),
            source_stamp,
            byte_size: metadata.len(),
            sha256,
            document,
            notes,
            resource_count,
            decrypted_enex_path: PathBuf::new(),
        })
    }

    fn discovered_candidates(
        &self,
        session_id: &CollectionSessionId,
    ) -> Result<Vec<DiscoveredCandidate>, ApplicationError> {
        let mut candidates = Vec::with_capacity(self.notes.len() + 1);
        let export = self.export_envelope()?;
        candidates.push(DiscoveredCandidate {
            summary: export_summary(session_id, self, &export),
            prefetched: Some(export),
        });
        for note in &self.notes {
            let envelope = self.note_envelope(note.ordinal)?;
            candidates.push(DiscoveredCandidate {
                summary: note_summary(session_id, self, note),
                prefetched: Some(envelope),
            });
        }
        Ok(candidates)
    }

    fn export_envelope(&self) -> Result<CandidateEnvelope, ApplicationError> {
        let payload = serde_json::to_string_pretty(&serde_json::json!({
            "schema": "evernote_notes_batch_manifest_v1",
            "adapter_version": ADAPTER_VERSION,
            "export_sha256": self.sha256.as_str(),
            "export_byte_size": self.byte_size,
            "export_application": self.document.application,
            "export_version": self.document.version,
            "export_date": self.document.export_date,
            "note_count": self.notes.len(),
            "resource_count": self.resource_count,
            "notes": self.notes.iter().map(|note| serde_json::json!({
                "ordinal": note.ordinal,
                "native_id": note.native_id,
                "payload_sha256": note.payload_sha256.as_str(),
                "resource_count": note.resources.len(),
                "resource_sha256": note.resources.iter().map(|resource| resource.sha256.as_str()).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
        }))
        .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
        let source_reference = self.path.to_string_lossy().into_owned();
        let common_metadata = export_common_metadata(self);
        Ok(CandidateEnvelope {
            protocol_version: "1".to_owned(),
            route_id: SourceRouteId(ROUTE_ID.to_owned()),
            source_reference,
            content_type: ContentType::Archive,
            payload_sha256: Sha256::of_bytes(payload.as_bytes()),
            metadata: Metadata::parse(
                &serde_json::json!({
                    "title": export_title(&self.path),
                    "import_format": "yinxiang_notes_export",
                    "adapter_version": ADAPTER_VERSION,
                    "export_sha256": self.sha256.as_str(),
                    "export_byte_size": self.byte_size,
                    "note_count": self.notes.len(),
                    "resource_count": self.resource_count,
                    "content_fingerprint": self.sha256.as_str(),
                })
                .to_string(),
            )?,
            payload: CandidatePayload::Text { text: payload },
            context: common_metadata.context.clone(),
            native_id: Some(format!("export:{}", self.sha256)),
            common_metadata,
        })
    }

    fn note_envelope(&self, ordinal: usize) -> Result<CandidateEnvelope, ApplicationError> {
        let parsed = self.note(ordinal)?;
        let note = &self.document.notes[ordinal - 1];
        let common_metadata = note_common_metadata(self, parsed, note);
        let metadata = Metadata::parse(
            &serde_json::json!({
                "title": note.title,
                "import_format": "yinxiang_notes_export",
                "adapter_version": ADAPTER_VERSION,
                "export_sha256": self.sha256.as_str(),
                "export_byte_size": self.byte_size,
                "export_application": self.document.application,
                "export_version": self.document.version,
                "export_date": self.document.export_date,
                "note_ordinal": ordinal,
                "created": note.created,
                "tags": note.tags,
                "note_attributes": note.attributes,
                "resources": parsed.resources,
                "content_fingerprint": parsed.payload_sha256.as_str(),
                "identity_scope": "immutable_export_and_note_ordinal",
            })
            .to_string(),
        )?;
        Ok(CandidateEnvelope {
            protocol_version: "1".to_owned(),
            route_id: SourceRouteId(ROUTE_ID.to_owned()),
            source_reference: note_location(&self.path, ordinal),
            content_type: ContentType::Document,
            payload_sha256: parsed.payload_sha256.clone(),
            metadata,
            payload: CandidatePayload::Text {
                text: note.content.value.clone(),
            },
            context: common_metadata.context.clone(),
            native_id: Some(parsed.native_id.clone()),
            common_metadata,
        })
    }

    fn note(&self, ordinal: usize) -> Result<&ParsedNote, ApplicationError> {
        if ordinal == 0 {
            return Err(ApplicationError::Integrity(
                "Evernote note ordinal must be positive".to_owned(),
            ));
        }
        self.notes
            .get(ordinal - 1)
            .ok_or_else(|| ApplicationError::NotFound(format!("Evernote note ordinal {ordinal}")))
    }

    fn materialize_note_resources(
        &self,
        runtime_root: &Path,
        ordinal: usize,
    ) -> Result<Vec<CaptureImportAsset>, ApplicationError> {
        let parsed = self.note(ordinal)?;
        let note = &self.document.notes[ordinal - 1];
        let output_root = runtime_root
            .join(self.sha256.as_str())
            .join(format!("note-{ordinal:06}"));
        fs::create_dir_all(&output_root).map_err(asset_io)?;
        note.resources
            .iter()
            .zip(&parsed.resources)
            .map(|(resource, parsed)| {
                let bytes = decode_base64(&resource.data.value).map_err(|error| {
                    ApplicationError::Integrity(format!(
                        "Evernote note {ordinal} resource {} base64 is invalid: {error}",
                        parsed.ordinal
                    ))
                })?;
                if Sha256::of_bytes(&bytes) != parsed.sha256 {
                    return Err(ApplicationError::Integrity(format!(
                        "Evernote note {ordinal} resource {} changed after discovery",
                        parsed.ordinal
                    )));
                }
                let filename = materialized_filename(parsed);
                let output = output_root.join(filename);
                if output.exists() {
                    let existing = fs::read(&output).map_err(asset_io)?;
                    if Sha256::of_bytes(&existing) != parsed.sha256 {
                        return Err(ApplicationError::Integrity(
                            "Evernote runtime resource hash mismatch".to_owned(),
                        ));
                    }
                } else {
                    fs::write(&output, bytes).map_err(asset_io)?;
                }
                Ok(CaptureImportAsset {
                    path: output.to_string_lossy().into_owned(),
                    role: AssetRole::Attachment,
                    expected_sha256: None,
                })
            })
            .collect()
    }
}

fn parse_note(
    note: &mut EnNote,
    ordinal: usize,
    export_sha: &str,
) -> Result<ParsedNote, ApplicationError> {
    validate_required("title", &note.title)?;
    validate_required("created", &note.created)?;
    if note.content.encoding.as_deref() != Some("base64:aes") {
        return Err(ApplicationError::Integrity(format!(
            "Evernote note {ordinal} content is not base64:aes"
        )));
    }
    let decrypted = decrypt_content(&note.content.value).map_err(|error| {
        ApplicationError::Integrity(format!(
            "Evernote note {ordinal} authentication/decryption failed: {error}"
        ))
    })?;
    validate_required("decrypted content", &decrypted)?;
    note.content.encoding = None;
    note.content.value = decrypted;
    let resources = note
        .resources
        .iter()
        .enumerate()
        .map(|(resource_index, resource)| ParsedResource::from_export(resource_index + 1, resource))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ParsedNote {
        ordinal,
        native_id: note_native_id(export_sha, ordinal),
        payload_sha256: Sha256::of_bytes(note.content.value.as_bytes()),
        created_at: parse_enex_timestamp(&note.created)?,
        resources,
    })
}

#[derive(Debug, Clone)]
struct ParsedNote {
    ordinal: usize,
    native_id: String,
    payload_sha256: Sha256,
    created_at: UtcTimestamp,
    resources: Vec<ParsedResource>,
}

#[derive(Debug, Clone, Serialize)]
struct ParsedResource {
    ordinal: usize,
    sha256: Sha256,
    byte_size: usize,
    mime: String,
    file_name: Option<String>,
    timestamp: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    duration: Option<u64>,
    recognition_sha256: Option<Sha256>,
}

impl ParsedResource {
    fn from_export(ordinal: usize, resource: &EnResource) -> Result<Self, ApplicationError> {
        if resource.data.encoding != "base64" {
            return Err(ApplicationError::Integrity(format!(
                "Evernote resource {ordinal} encoding is not base64"
            )));
        }
        validate_required("resource mime", &resource.mime)?;
        let bytes = decode_base64(&resource.data.value).map_err(|error| {
            ApplicationError::Integrity(format!(
                "Evernote resource {ordinal} base64 is invalid: {error}"
            ))
        })?;
        Ok(Self {
            ordinal,
            sha256: Sha256::of_bytes(&bytes),
            byte_size: bytes.len(),
            mime: resource.mime.clone(),
            file_name: resource.attributes.file_name.clone(),
            timestamp: resource.attributes.timestamp.clone(),
            width: resource.width,
            height: resource.height,
            duration: resource.duration,
            recognition_sha256: resource
                .recognition
                .as_deref()
                .map(|value| Sha256::of_bytes(value.as_bytes())),
        })
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename = "en-export")]
struct EnExport {
    #[serde(rename = "@export-date")]
    export_date: String,
    #[serde(rename = "@application")]
    application: String,
    #[serde(rename = "@version")]
    version: String,
    #[serde(rename = "note", default)]
    notes: Vec<EnNote>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct EnNote {
    title: String,
    content: EnContent,
    created: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    updated: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    deleted: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    active: Option<bool>,
    #[serde(rename = "tag", default)]
    tags: Vec<String>,
    #[serde(rename = "note-attributes", default)]
    attributes: EnNoteAttributes,
    #[serde(rename = "resource", default)]
    resources: Vec<EnResource>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct EnContent {
    #[serde(rename = "@encoding", default, skip_serializing_if = "Option::is_none")]
    encoding: Option<String>,
    #[serde(rename = "$text", default)]
    value: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
struct EnNoteAttributes {
    #[serde(rename = "subject-date", skip_serializing_if = "Option::is_none")]
    subject_date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    latitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    longitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    altitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<String>,
    #[serde(rename = "source-url", skip_serializing_if = "Option::is_none")]
    source_url: Option<String>,
    #[serde(rename = "source-application", skip_serializing_if = "Option::is_none")]
    source_application: Option<String>,
    #[serde(rename = "share-date", skip_serializing_if = "Option::is_none")]
    share_date: Option<String>,
    #[serde(rename = "reminder-order", skip_serializing_if = "Option::is_none")]
    reminder_order: Option<i64>,
    #[serde(rename = "reminder-time", skip_serializing_if = "Option::is_none")]
    reminder_time: Option<String>,
    #[serde(rename = "reminder-done-time", skip_serializing_if = "Option::is_none")]
    reminder_done_time: Option<String>,
    #[serde(rename = "place-name", skip_serializing_if = "Option::is_none")]
    place_name: Option<String>,
    #[serde(rename = "content-class", skip_serializing_if = "Option::is_none")]
    content_class: Option<String>,
    #[serde(rename = "application-data", default)]
    application_data: Vec<EnApplicationData>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct EnApplicationData {
    #[serde(rename = "@key")]
    key: String,
    #[serde(rename = "$text", default)]
    value: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct EnResource {
    data: EnResourceData,
    mime: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    width: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    height: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    duration: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    recognition: Option<String>,
    #[serde(
        rename = "alternate-data",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    alternate_data: Option<EnResourceData>,
    #[serde(rename = "resource-attributes", default)]
    attributes: EnResourceAttributes,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct EnResourceData {
    #[serde(rename = "@encoding")]
    encoding: String,
    #[serde(rename = "$text", default)]
    value: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(default)]
struct EnResourceAttributes {
    #[serde(rename = "source-url", skip_serializing_if = "Option::is_none")]
    source_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timestamp: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    latitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    longitude: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    altitude: Option<f64>,
    #[serde(rename = "camera-make", skip_serializing_if = "Option::is_none")]
    camera_make: Option<String>,
    #[serde(rename = "camera-model", skip_serializing_if = "Option::is_none")]
    camera_model: Option<String>,
    #[serde(rename = "client-will-index", skip_serializing_if = "Option::is_none")]
    client_will_index: Option<bool>,
    #[serde(rename = "file-name", skip_serializing_if = "Option::is_none")]
    file_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    attachment: Option<bool>,
}

fn decrypt_content(value: &str) -> Result<String, &'static str> {
    let bytes = decode_base64(value).map_err(|_| "invalid base64")?;
    if bytes.len() < ENC0_OVERHEAD_BYTES + 16 || &bytes[..4] != b"ENC0" {
        return Err("invalid ENC0 package");
    }
    let encrypted_length = bytes.len() - ENC0_OVERHEAD_BYTES;
    if encrypted_length == 0 || encrypted_length % 16 != 0 {
        return Err("invalid AES-CBC payload length");
    }
    let nonce_one: [u8; 16] = bytes[4..20].try_into().map_err(|_| "invalid nonce")?;
    let nonce_two: [u8; 16] = bytes[20..36].try_into().map_err(|_| "invalid nonce")?;
    let iv: [u8; 16] = bytes[36..52].try_into().map_err(|_| "invalid IV")?;
    let encrypted_end = bytes.len() - 32;
    let expected_mac = &bytes[encrypted_end..];
    let encryption_key = derive_key(&nonce_one);
    let authentication_key = derive_key(&nonce_two);
    HmacSha256::new_from_slice(&authentication_key)
        .map_err(|_| "invalid authentication key")?
        .chain_update(&bytes[..encrypted_end])
        .verify_slice(expected_mac)
        .map_err(|_| "HMAC verification failed")?;
    let mut encrypted = bytes[52..encrypted_end].to_vec();
    let plaintext = Aes128CbcDecryptor::new((&encryption_key).into(), (&iv).into())
        .decrypt_padded_mut::<Pkcs7>(&mut encrypted)
        .map_err(|_| "AES-CBC padding verification failed")?;
    let content = plaintext
        .strip_suffix(&[0])
        .ok_or("decrypted content has no terminator")?;
    String::from_utf8(content.to_vec()).map_err(|_| "decrypted content is not UTF-8")
}

fn derive_key(nonce: &[u8; 16]) -> [u8; 16] {
    let mut initial = [0u8; 20];
    initial[..16].copy_from_slice(nonce);
    initial[19] = 1;
    let mut state = [0u8; 32];
    let mut key = [0u8; 16];
    for round in 0..50_000 {
        let digest = if round == 0 {
            HmacSha256::new_from_slice(HMAC_SEED)
                .expect("static HMAC seed is valid")
                .chain_update(initial)
                .finalize()
                .into_bytes()
        } else {
            HmacSha256::new_from_slice(HMAC_SEED)
                .expect("static HMAC seed is valid")
                .chain_update(state)
                .finalize()
                .into_bytes()
        };
        state.copy_from_slice(&digest);
        for (target, value) in key.iter_mut().zip(&state[..16]) {
            *target ^= value;
        }
    }
    key
}

fn decode_base64(value: &str) -> Result<Vec<u8>, base64::DecodeError> {
    if value.bytes().any(|byte| byte.is_ascii_whitespace()) {
        let compact = value
            .bytes()
            .filter(|byte| !byte.is_ascii_whitespace())
            .collect::<Vec<_>>();
        BASE64.decode(compact)
    } else {
        BASE64.decode(value)
    }
}

fn canonical_notes_path(path: &Path) -> Result<PathBuf, ApplicationError> {
    if !path.is_absolute() {
        return Err(ApplicationError::Conflict(
            "Evernote .notes source path must be absolute".to_owned(),
        ));
    }
    if !path
        .extension()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.eq_ignore_ascii_case("notes"))
    {
        return Err(ApplicationError::Conflict(
            "Evernote source must have the .notes extension".to_owned(),
        ));
    }
    fs::canonicalize(path).map_err(asset_io)
}

fn source_stamp(path: &Path) -> Result<SourceStamp, ApplicationError> {
    let metadata = fs::metadata(path).map_err(asset_io)?;
    Ok(SourceStamp {
        byte_size: metadata.len(),
        modified: metadata.modified().map_err(asset_io)?,
    })
}

fn parse_candidate_location(value: &str) -> Result<(PathBuf, Option<usize>), ApplicationError> {
    if let Some((path, ordinal)) = value.rsplit_once("#note=") {
        let ordinal = ordinal.parse::<usize>().map_err(|_| {
            ApplicationError::Integrity("Evernote candidate note ordinal is invalid".to_owned())
        })?;
        Ok((canonical_notes_path(Path::new(path))?, Some(ordinal)))
    } else {
        Ok((canonical_notes_path(Path::new(value))?, None))
    }
}

fn note_native_id(export_sha: &str, ordinal: usize) -> String {
    format!("export:{export_sha}:note:{ordinal:06}")
}

fn note_location(path: &Path, ordinal: usize) -> String {
    format!("{}#note={ordinal:06}", path.to_string_lossy())
}

fn export_candidate_id(sha256: &Sha256) -> String {
    format!("evernote_export_{}", &sha256.as_str()[..24])
}

fn note_candidate_id(sha256: &Sha256, ordinal: usize) -> String {
    format!("evernote_{}_{ordinal:06}", &sha256.as_str()[..16])
}

fn export_title(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("Evernote export.notes")
        .to_owned()
}

fn export_common_metadata(export: &ParsedExport) -> CommonSourceMetadata {
    let title = export_title(&export.path);
    CommonSourceMetadata {
        title: Some(title.clone()),
        hierarchy: vec![SourceHierarchyNode {
            kind: Some("official_export".to_owned()),
            name: title.clone(),
            native_id: Some(export.sha256.as_str().to_owned()),
            locator: Some(export.path.to_string_lossy().into_owned()),
        }],
        context: Some(format!("Official Yinxiang .notes export / {title}")),
        limitations: common_limitations(),
        access_state: SourceAccessState::Accessible,
        ..CommonSourceMetadata::default()
    }
}

fn note_common_metadata(
    export: &ParsedExport,
    parsed: &ParsedNote,
    note: &EnNote,
) -> CommonSourceMetadata {
    let export_title = export_title(&export.path);
    CommonSourceMetadata {
        title: Some(note.title.clone()),
        source_published_at: Some(parsed.created_at.clone()),
        hierarchy: vec![
            SourceHierarchyNode {
                kind: Some("official_export".to_owned()),
                name: export_title.clone(),
                native_id: Some(export.sha256.as_str().to_owned()),
                locator: Some(export.path.to_string_lossy().into_owned()),
            },
            SourceHierarchyNode {
                kind: Some("note".to_owned()),
                name: note.title.clone(),
                native_id: Some(parsed.native_id.clone()),
                locator: Some(note_location(&export.path, parsed.ordinal)),
            },
        ],
        context: Some(format!(
            "Official Yinxiang .notes export / note {:06}",
            parsed.ordinal
        )),
        limitations: common_limitations(),
        access_state: SourceAccessState::Accessible,
        media: SourceMediaMetadata {
            schema: babata_domain::SOURCE_MEDIA_METADATA_SCHEMA_V1.to_owned(),
            entries: parsed
                .resources
                .iter()
                .map(|resource| SourceMediaEntry {
                    kind: "attachment".to_owned(),
                    media_type: Some(resource.mime.clone()),
                    duration_ms: resource.duration,
                    width: resource.width,
                    height: resource.height,
                    page_count: None,
                })
                .collect(),
        },
        ..CommonSourceMetadata::default()
    }
}

fn common_limitations() -> Vec<SourceLimitation> {
    vec![
        SourceLimitation {
            code: "export_scoped_identity".to_owned(),
            detail: "the .notes export has no note GUID or update timestamp; identity is the immutable export hash plus note ordinal".to_owned(),
        },
        SourceLimitation {
            code: "notebook_hierarchy_unavailable".to_owned(),
            detail: "the official .notes export does not include notebook hierarchy".to_owned(),
        },
    ]
}

fn export_summary(
    session_id: &CollectionSessionId,
    export: &ParsedExport,
    envelope: &CandidateEnvelope,
) -> CandidateSummary {
    let title = export_title(&export.path);
    CandidateSummary {
        candidate_id: export_candidate_id(&export.sha256),
        session_id: session_id.clone(),
        route_id: SourceRouteId(ROUTE_ID.to_owned()),
        source_native_id: envelope.native_id.clone(),
        title: Some(title.clone()),
        source_location: Some(export.path.to_string_lossy().into_owned()),
        hierarchy: vec![title],
        content_type: ContentType::Archive,
        source_updated_at: None,
        attachment_available: Some(true),
        limitations: common_limitations()
            .iter()
            .map(|limitation| limitation.detail.clone())
            .collect(),
        selection_capabilities: vec![
            "batch_export".to_owned(),
            "explicit_export_scope".to_owned(),
        ],
        common_metadata: envelope.common_metadata.clone(),
    }
}

fn note_summary(
    session_id: &CollectionSessionId,
    export: &ParsedExport,
    note: &ParsedNote,
) -> CandidateSummary {
    let source = &export.document.notes[note.ordinal - 1];
    let common_metadata = note_common_metadata(export, note, source);
    CandidateSummary {
        candidate_id: note_candidate_id(&export.sha256, note.ordinal),
        session_id: session_id.clone(),
        route_id: SourceRouteId(ROUTE_ID.to_owned()),
        source_native_id: Some(note.native_id.clone()),
        title: Some(source.title.clone()),
        source_location: Some(note_location(&export.path, note.ordinal)),
        hierarchy: common_metadata
            .hierarchy
            .iter()
            .map(|node| node.name.clone())
            .collect(),
        content_type: ContentType::Document,
        source_updated_at: None,
        attachment_available: Some(!note.resources.is_empty()),
        limitations: common_limitations()
            .iter()
            .map(|limitation| limitation.detail.clone())
            .collect(),
        selection_capabilities: vec![
            "single".to_owned(),
            "visible_set".to_owned(),
            "explicit_export_scope".to_owned(),
        ],
        common_metadata,
    }
}

fn write_decrypted_enex(
    runtime_root: &Path,
    export: &ParsedExport,
) -> Result<PathBuf, ApplicationError> {
    let output_root = runtime_root.join(export.sha256.as_str());
    fs::create_dir_all(&output_root).map_err(asset_io)?;
    let output = output_root.join("decrypted.enex");
    let body = quick_xml::se::to_string(&export.document).map_err(|error| {
        ApplicationError::Integrity(format!("unable to serialize decrypted ENEX: {error}"))
    })?;
    let document = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE en-export SYSTEM \"http://xml.evernote.com/pub/evernote-export4.dtd\">\n{body}\n"
    );
    if output.exists() {
        let current = fs::read(&output).map_err(asset_io)?;
        if Sha256::of_bytes(&current) == Sha256::of_bytes(document.as_bytes()) {
            return Ok(output);
        }
    }
    fs::write(&output, document).map_err(asset_io)?;
    Ok(output)
}

fn parse_enex_timestamp(value: &str) -> Result<UtcTimestamp, ApplicationError> {
    let bytes = value.as_bytes();
    if bytes.len() != 16
        || bytes[8] != b'T'
        || bytes[15] != b'Z'
        || bytes
            .iter()
            .enumerate()
            .any(|(index, byte)| !matches!(index, 8 | 15) && !byte.is_ascii_digit())
    {
        return Err(ApplicationError::Integrity(
            "Evernote timestamp is not YYYYMMDDTHHMMSSZ".to_owned(),
        ));
    }
    UtcTimestamp::parse(format!(
        "{}-{}-{}T{}:{}:{}Z",
        &value[0..4],
        &value[4..6],
        &value[6..8],
        &value[9..11],
        &value[11..13],
        &value[13..15]
    ))
    .map_err(Into::into)
}

fn reject_entity_declarations(bytes: &[u8]) -> Result<(), ApplicationError> {
    if bytes
        .windows(b"<!ENTITY".len())
        .any(|window| window.eq_ignore_ascii_case(b"<!ENTITY"))
    {
        return Err(ApplicationError::Integrity(
            "Evernote XML entity declarations are not allowed".to_owned(),
        ));
    }
    Ok(())
}

fn validate_required(field: &str, value: &str) -> Result<(), ApplicationError> {
    if value.trim().is_empty() {
        Err(ApplicationError::Integrity(format!(
            "Evernote {field} is empty"
        )))
    } else {
        Ok(())
    }
}

fn materialized_filename(resource: &ParsedResource) -> String {
    let original = resource
        .file_name
        .as_deref()
        .unwrap_or("attachment.bin")
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>();
    format!(
        "resource-{:03}-{}-{original}",
        resource.ordinal,
        &resource.sha256.as_str()[..12]
    )
}

fn asset_io(error: std::io::Error) -> ApplicationError {
    ApplicationError::Asset(format!("Evernote filesystem {:?} failure", error.kind()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use aes::cipher::{BlockEncryptMut, block_padding::Pkcs7};
    use babata_application::{
        CollectorSessionService, StartCollectionCommand, ports::RawRepositoryPort,
    };
    use babata_domain::{CollectionItemState, CollectionSelection, RecollectionState};
    use tempfile::tempdir;

    use crate::{
        FileAssetStore, SystemClock,
        paths::{DataPaths, ensure_layout},
    };

    type Aes128CbcEncryptor = cbc::Encryptor<Aes128>;

    #[test]
    fn valid_enc0_content_is_authenticated_and_decrypted() {
        let encoded = encrypt_fixture("<en-note>fixture body</en-note>");
        assert_eq!(
            decrypt_content(&encoded).unwrap(),
            "<en-note>fixture body</en-note>"
        );
    }

    #[test]
    fn tampered_and_truncated_enc0_packages_are_rejected() {
        let encoded = encrypt_fixture("<en-note>fixture body</en-note>");
        let mut tampered = BASE64.decode(encoded.as_bytes()).unwrap();
        tampered[60] ^= 1;
        assert_eq!(
            decrypt_content(&BASE64.encode(tampered)),
            Err("HMAC verification failed")
        );
        let mut truncated = BASE64.decode(encoded.as_bytes()).unwrap();
        truncated.truncate(80);
        assert!(decrypt_content(&BASE64.encode(truncated)).is_err());
    }

    #[test]
    fn notes_export_discovers_batch_and_note_without_writing_c0() {
        let temporary = tempdir().unwrap();
        let export_path = temporary.path().join("fixture.notes");
        fs::write(&export_path, fixture_export()).unwrap();
        let runtime = temporary.path().join("runtime");
        let adapter = EvernoteNotesAdapter::new(runtime.clone());
        let session_id = CollectionSessionId::new();
        let candidates = adapter
            .discover(
                &session_id,
                &format!("notes:{}", export_path.canonicalize().unwrap().display()),
            )
            .unwrap();
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].summary.content_type, ContentType::Archive);
        assert_eq!(candidates[1].summary.title.as_deref(), Some("Fixture note"));
        assert_eq!(
            candidates[1]
                .summary
                .common_metadata
                .source_published_at
                .as_ref()
                .map(UtcTimestamp::as_str),
            Some("2026-07-19T01:02:03Z")
        );
        let generated = fs::read_dir(&runtime)
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path()
            .join("decrypted.enex");
        let generated = fs::read_to_string(generated).unwrap();
        assert!(generated.contains("fixture body"));
        assert!(!generated.contains("base64:aes"));
    }

    #[test]
    fn note_collection_materializes_only_explicitly_requested_resources() {
        let temporary = tempdir().unwrap();
        let export_path = temporary.path().join("fixture.notes");
        fs::write(&export_path, fixture_export()).unwrap();
        let adapter = EvernoteNotesAdapter::new(temporary.path().join("runtime"));
        let candidates = adapter
            .discover(
                &CollectionSessionId::new(),
                &format!("notes:{}", export_path.canonicalize().unwrap().display()),
            )
            .unwrap();
        let note = &candidates[1];
        let without_assets = adapter
            .collect(&note.summary, note.prefetched.as_ref(), false)
            .unwrap();
        assert!(matches!(
            without_assets,
            AcquisitionOutcome::Found { ref assets, .. } if assets.is_empty()
        ));
        let with_assets = adapter
            .collect(&note.summary, note.prefetched.as_ref(), true)
            .unwrap();
        let AcquisitionOutcome::Found { candidate, assets } = with_assets else {
            panic!("expected found outcome");
        };
        assert_eq!(assets.len(), 1);
        assert_eq!(fs::read(&assets[0].path).unwrap(), b"resource bytes");
        assert_eq!(candidate.native_id, note.summary.source_native_id);
    }

    #[test]
    fn source_replacement_after_discovery_is_not_hidden_by_the_cache() {
        let temporary = tempdir().unwrap();
        let export_path = temporary.path().join("fixture.notes");
        fs::write(&export_path, fixture_export()).unwrap();
        let adapter = EvernoteNotesAdapter::new(temporary.path().join("runtime"));
        let candidates = adapter
            .discover(
                &CollectionSessionId::new(),
                &format!("notes:{}", export_path.canonicalize().unwrap().display()),
            )
            .unwrap();
        fs::write(
            &export_path,
            fixture_export().replace("Fixture note", "Changed fixture note"),
        )
        .unwrap();

        let error = adapter
            .collect(
                &candidates[1].summary,
                candidates[1].prefetched.as_ref(),
                false,
            )
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("export changed after candidate discovery")
        );
    }

    #[test]
    fn relative_scope_entity_declaration_and_bad_or_missing_resource_are_rejected() {
        let temporary = tempdir().unwrap();
        let adapter = EvernoteNotesAdapter::new(temporary.path().join("runtime"));
        assert!(
            adapter
                .discover(&CollectionSessionId::new(), "notes:relative.notes")
                .is_err()
        );

        let entity_path = temporary.path().join("entity.notes");
        fs::write(
            &entity_path,
            fixture_export().replace(
                "<!DOCTYPE en-export SYSTEM \"http://xml.evernote.com/pub/evernote-export4.dtd\">",
                "<!DOCTYPE en-export [<!ENTITY xxe SYSTEM \"file:///etc/passwd\">]>",
            ),
        )
        .unwrap();
        assert!(
            adapter
                .discover(
                    &CollectionSessionId::new(),
                    &format!("notes:{}", entity_path.canonicalize().unwrap().display()),
                )
                .is_err()
        );

        let resource_path = temporary.path().join("resource.notes");
        fs::write(
            &resource_path,
            fixture_export().replace("cmVzb3VyY2UgYnl0ZXM=", "not-base64!"),
        )
        .unwrap();
        assert!(
            adapter
                .discover(
                    &CollectionSessionId::new(),
                    &format!("notes:{}", resource_path.canonicalize().unwrap().display()),
                )
                .is_err()
        );

        let missing_resource_path = temporary.path().join("missing-resource.notes");
        fs::write(
            &missing_resource_path,
            fixture_export().replace("<data encoding=\"base64\">cmVzb3VyY2UgYnl0ZXM=</data>", ""),
        )
        .unwrap();
        assert!(
            adapter
                .discover(
                    &CollectionSessionId::new(),
                    &format!(
                        "notes:{}",
                        missing_resource_path.canonicalize().unwrap().display()
                    ),
                )
                .is_err()
        );
    }

    #[test]
    fn collector_persists_export_and_note_then_recollects_unchanged() {
        let temporary = tempdir().unwrap();
        let export_path = temporary.path().join("fixture.notes");
        fs::write(&export_path, fixture_export()).unwrap();
        let paths = DataPaths::new(temporary.path().join("data"));
        ensure_layout(&paths).unwrap();
        let repository = crate::open_collection_database(&paths, 100).unwrap();
        let service = CollectorSessionService::new(
            repository.clone(),
            FileAssetStore::new(paths.clone()),
            SystemClock,
            vec![Box::new(EvernoteNotesAdapter::new(
                paths.root().join("04_runtime/provider-downloads/evernote"),
            ))],
        );
        let session = service
            .start(StartCollectionCommand {
                route_id: SourceRouteId(ROUTE_ID.to_owned()),
                source_reference: format!(
                    "notes:{}",
                    export_path.canonicalize().unwrap().display()
                ),
                scope_description: "one synthetic export".to_owned(),
                authorisation_id: "fixture-authorisation".to_owned(),
            })
            .unwrap();
        let candidates = service.candidates(&session.session_id).unwrap();
        let items = service
            .select(CollectionSelection {
                session_id: session.session_id.clone(),
                candidate_ids: candidates
                    .iter()
                    .map(|candidate| candidate.candidate_id.clone())
                    .collect(),
                scope_description: "the discovered synthetic export".to_owned(),
                confirmed: true,
                authorised_context: "fixture-authorisation".to_owned(),
                requested_attachments: true,
            })
            .unwrap();
        assert_eq!(items.len(), 2);
        assert!(
            items
                .iter()
                .all(|item| item.state == CollectionItemState::Saved),
            "{items:#?}"
        );
        let export_item = items
            .iter()
            .find(|item| item.candidate_id.starts_with("evernote_export_"))
            .unwrap();
        let note_item = items
            .iter()
            .find(|item| !item.candidate_id.starts_with("evernote_export_"))
            .unwrap();
        let export_detail = repository
            .load_detail(export_item.item_id.as_ref().unwrap())
            .unwrap();
        let note_detail = repository
            .load_detail(note_item.item_id.as_ref().unwrap())
            .unwrap();
        assert_eq!(export_detail.revisions.len(), 1);
        assert_eq!(export_detail.assets.len(), 2);
        assert!(
            export_detail
                .assets
                .iter()
                .any(|asset| asset.role == AssetRole::Original)
        );
        assert!(
            export_detail
                .assets
                .iter()
                .any(|asset| asset.role == AssetRole::Export)
        );
        assert_eq!(note_detail.revisions.len(), 1);
        assert_eq!(note_detail.assets.len(), 1);
        assert_eq!(note_detail.assets[0].role, AssetRole::Attachment);
        assert!(
            note_detail.revisions[0]
                .raw_text
                .as_deref()
                .unwrap()
                .contains("fixture body")
        );

        let recollected = service.recollect_session(&session.session_id).unwrap();
        assert_eq!(recollected.len(), 2);
        for outcome in recollected {
            assert_eq!(outcome.state, RecollectionState::Unchanged);
            assert!(outcome.new_revision_id.is_none());
            assert_eq!(
                repository
                    .load_detail(&outcome.item_id)
                    .unwrap()
                    .revisions
                    .len(),
                1
            );
        }
    }

    fn encrypt_fixture(content: &str) -> String {
        let nonce_one = [1u8; 16];
        let nonce_two = [2u8; 16];
        let iv = [3u8; 16];
        let encryption_key = derive_key(&nonce_one);
        let authentication_key = derive_key(&nonce_two);
        let mut plaintext = content.as_bytes().to_vec();
        plaintext.push(0);
        let encrypted = Aes128CbcEncryptor::new((&encryption_key).into(), (&iv).into())
            .encrypt_padded_vec_mut::<Pkcs7>(&plaintext);
        let mut package = Vec::new();
        package.extend_from_slice(b"ENC0");
        package.extend_from_slice(&nonce_one);
        package.extend_from_slice(&nonce_two);
        package.extend_from_slice(&iv);
        package.extend_from_slice(&encrypted);
        let mac = HmacSha256::new_from_slice(&authentication_key)
            .unwrap()
            .chain_update(&package)
            .finalize()
            .into_bytes();
        package.extend_from_slice(&mac);
        BASE64.encode(package)
    }

    fn fixture_export() -> String {
        format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE en-export SYSTEM "http://xml.evernote.com/pub/evernote-export4.dtd">
<en-export export-date="20260719T010203Z" application="Evernote/Windows" version="6.x">
  <note>
    <title>Fixture note</title>
    <content encoding="base64:aes">{}</content>
    <created>20260719T010203Z</created>
    <note-attributes><source>fixture</source><content-class>fixture.note</content-class><application-data key="fixture.key">fixture value</application-data></note-attributes>
    <resource>
      <data encoding="base64">cmVzb3VyY2UgYnl0ZXM=</data>
      <mime>application/octet-stream</mime>
      <resource-attributes><file-name>fixture.bin</file-name><timestamp>20260719T010203Z</timestamp></resource-attributes>
    </resource>
  </note>
</en-export>"#,
            encrypt_fixture("<en-note>fixture body</en-note>")
        )
    }
}
