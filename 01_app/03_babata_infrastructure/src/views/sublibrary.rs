use std::{fs, io, path::PathBuf};

use babata_application::{ApplicationError, ports::SublibraryMaterializerPort};
use babata_domain::{
    Sha256, SublibraryId, SublibraryMaterialization, SublibraryMaterializationDocument,
    SublibraryMaterializationState, UtcTimestamp,
};
use serde::{Deserialize, Serialize};

use crate::paths::{DataPaths, ensure_layout};

const SCHEMA_VERSION: &str = "babata.sublibrary-materialization/v1";

#[derive(Debug, Clone)]
pub struct SublibraryViewStore {
    paths: DataPaths,
}

impl SublibraryViewStore {
    pub fn new(paths: DataPaths) -> Self {
        Self { paths }
    }

    fn root(&self) -> PathBuf {
        self.paths.root().join("03_views/sublibraries")
    }

    fn canonical_root(&self) -> Result<PathBuf, ApplicationError> {
        let canonical_data_root = self.paths.root().canonicalize().map_err(io_error)?;
        let views = self.paths.root().join("03_views");
        let canonical_views = views.canonicalize().map_err(io_error)?;
        let canonical_root = self.root().canonicalize().map_err(io_error)?;
        if canonical_views.parent() != Some(canonical_data_root.as_path())
            || canonical_root.parent() != Some(canonical_views.as_path())
        {
            return Err(ApplicationError::Integrity(
                "sublibrary C2 root escaped BABATA_DATA_HOME".to_owned(),
            ));
        }
        Ok(canonical_root)
    }

    fn directory(
        &self,
        sublibrary_id: &SublibraryId,
        version: u32,
    ) -> Result<PathBuf, ApplicationError> {
        if version == 0 {
            return Err(ApplicationError::Integrity(
                "sublibrary version must be positive".to_owned(),
            ));
        }
        Ok(self
            .root()
            .join(sublibrary_id.as_str())
            .join(format!("v{version}")))
    }

    fn ensure_safe_existing_directory(
        &self,
        sublibrary_id: &SublibraryId,
        version: u32,
    ) -> Result<PathBuf, ApplicationError> {
        let directory = self.directory(sublibrary_id, version)?;
        if !directory.exists() {
            return Err(ApplicationError::NotFound(format!(
                "sublibrary materialization {sublibrary_id} v{version}"
            )));
        }
        let canonical_root = self.canonical_root()?;
        let canonical_directory = directory.canonicalize().map_err(io_error)?;
        if canonical_directory
            .parent()
            .and_then(|parent| parent.parent())
            != Some(canonical_root.as_path())
        {
            return Err(ApplicationError::Integrity(
                "sublibrary materialization escaped its controlled C2 root".to_owned(),
            ));
        }
        Ok(directory)
    }

    fn ensure_safe_parent(
        &self,
        sublibrary_id: &SublibraryId,
    ) -> Result<PathBuf, ApplicationError> {
        let root = self.root();
        fs::create_dir_all(&root).map_err(io_error)?;
        let canonical_root = self.canonical_root()?;
        let parent = root.join(sublibrary_id.as_str());
        if !parent.exists() {
            fs::create_dir(&parent).map_err(io_error)?;
        }
        let canonical_parent = parent.canonicalize().map_err(io_error)?;
        if canonical_parent.parent() != Some(canonical_root.as_path()) {
            return Err(ApplicationError::Integrity(
                "sublibrary escaped its controlled C2 root".to_owned(),
            ));
        }
        Ok(parent)
    }

    fn read_manifest(
        &self,
        sublibrary_id: &SublibraryId,
        version: u32,
    ) -> Result<MaterializationManifest, ApplicationError> {
        let directory = self.ensure_safe_existing_directory(sublibrary_id, version)?;
        let bytes = fs::read(directory.join("manifest.json")).map_err(io_error)?;
        let manifest: MaterializationManifest = serde_json::from_slice(&bytes)
            .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
        if manifest.schema_version != SCHEMA_VERSION
            || manifest.sublibrary_id != *sublibrary_id
            || manifest.definition_version != version
        {
            return Err(ApplicationError::Integrity(
                "sublibrary materialization manifest identity is invalid".to_owned(),
            ));
        }
        let relative_root = format!("03_views/sublibraries/{sublibrary_id}/v{version}");
        if manifest.materialization_path != format!("{relative_root}/materialization.json")
            || manifest.manifest_path != format!("{relative_root}/manifest.json")
            || manifest.files != ["materialization.json"]
            || manifest.member_count != manifest.inputs.len() as u64
        {
            return Err(ApplicationError::Integrity(
                "sublibrary materialization manifest paths or counts are invalid".to_owned(),
            ));
        }
        Sha256::parse(&manifest.definition_sha256)?;
        Sha256::parse(&manifest.output_sha256)?;
        Ok(manifest)
    }

    fn manifest(
        document: &SublibraryMaterializationDocument,
        output_sha256: &Sha256,
    ) -> Result<MaterializationManifest, ApplicationError> {
        let authority = document.definition.authority.as_ref().ok_or_else(|| {
            ApplicationError::Integrity(
                "sublibrary materialization requires a first-party C0 definition reference"
                    .to_owned(),
            )
        })?;
        let relative_root = format!(
            "03_views/sublibraries/{}/v{}",
            document.definition.id, document.definition.version
        );
        Ok(MaterializationManifest {
            schema_version: SCHEMA_VERSION.to_owned(),
            sublibrary_id: document.definition.id.clone(),
            definition_version: document.definition.version,
            definition_item_id: authority.item_id.to_string(),
            definition_revision_id: authority.revision_id.to_string(),
            definition_sha256: document.definition_sha256.to_string(),
            projection_fingerprint: document.projection_fingerprint.clone(),
            output_sha256: output_sha256.to_string(),
            member_count: u64::try_from(document.members.len()).map_err(|_| {
                ApplicationError::Integrity("too many sublibrary members".to_owned())
            })?,
            built_at: document.built_at.clone(),
            materialization_path: format!("{relative_root}/materialization.json"),
            manifest_path: format!("{relative_root}/manifest.json"),
            files: vec!["materialization.json".to_owned()],
            inputs: document
                .members
                .iter()
                .map(|member| MaterializationInput {
                    record_id: member.record.record_id.clone(),
                    item_id: member.record.item_id.as_ref().map(ToString::to_string),
                    revision_id: member.record.revision_id.as_ref().map(ToString::to_string),
                    semantic_id: member.record.semantic_id.clone(),
                    input_sha256: member.input_sha256.to_string(),
                    origin_kind: member.record.origin_kind.clone(),
                    review_state: member.record.review_state.clone(),
                    human_judgment: member.record.judgment.human_judgment,
                    confirmed_fact: member.record.judgment.confirmed_fact,
                    inclusion_reasons: member.inclusion_reasons.clone(),
                })
                .collect(),
            exclusions: document
                .exclusions
                .iter()
                .map(|excluded| format!("{}:{}", excluded.record_id, excluded.reason))
                .collect(),
            limitations: document.limitations.clone(),
        })
    }

    fn outcome(
        manifest: MaterializationManifest,
        state: SublibraryMaterializationState,
    ) -> Result<SublibraryMaterialization, ApplicationError> {
        Ok(SublibraryMaterialization {
            sublibrary_id: manifest.sublibrary_id,
            definition_version: manifest.definition_version,
            state,
            member_count: manifest.member_count,
            definition_sha256: Sha256::parse(manifest.definition_sha256)?,
            projection_fingerprint: manifest.projection_fingerprint,
            output_sha256: Sha256::parse(manifest.output_sha256)?,
            materialization_path: manifest.materialization_path,
            manifest_path: manifest.manifest_path,
            built_at: manifest.built_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct MaterializationManifest {
    schema_version: String,
    sublibrary_id: SublibraryId,
    definition_version: u32,
    definition_item_id: String,
    definition_revision_id: String,
    definition_sha256: String,
    projection_fingerprint: String,
    output_sha256: String,
    member_count: u64,
    built_at: UtcTimestamp,
    materialization_path: String,
    manifest_path: String,
    files: Vec<String>,
    inputs: Vec<MaterializationInput>,
    exclusions: Vec<String>,
    limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct MaterializationInput {
    record_id: String,
    item_id: Option<String>,
    revision_id: Option<String>,
    semantic_id: Option<String>,
    input_sha256: String,
    origin_kind: String,
    review_state: Option<String>,
    human_judgment: bool,
    confirmed_fact: bool,
    inclusion_reasons: Vec<String>,
}

impl SublibraryMaterializerPort for SublibraryViewStore {
    fn build(
        &self,
        document: &SublibraryMaterializationDocument,
    ) -> Result<SublibraryMaterialization, ApplicationError> {
        ensure_layout(&self.paths).map_err(io_error)?;
        let parent = self.ensure_safe_parent(&document.definition.id)?;
        let directory = self.directory(&document.definition.id, document.definition.version)?;
        if directory.exists() {
            self.ensure_safe_existing_directory(
                &document.definition.id,
                document.definition.version,
            )?;
        } else {
            debug_assert_eq!(directory.parent(), Some(parent.as_path()));
            fs::create_dir(&directory).map_err(io_error)?;
        }
        let materialization_bytes = serde_json::to_vec_pretty(document)
            .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
        let output_sha256 = Sha256::of_bytes(&materialization_bytes);
        let manifest = Self::manifest(document, &output_sha256)?;
        replace_file(
            directory.join("materialization.json"),
            &materialization_bytes,
        )?;
        replace_file(
            directory.join("manifest.json"),
            &serde_json::to_vec_pretty(&manifest)
                .map_err(|error| ApplicationError::Integrity(error.to_string()))?,
        )?;
        Self::outcome(manifest, SublibraryMaterializationState::Succeeded)
    }

    fn status(
        &self,
        sublibrary_id: &SublibraryId,
        version: u32,
    ) -> Result<SublibraryMaterialization, ApplicationError> {
        Self::outcome(
            self.read_manifest(sublibrary_id, version)?,
            SublibraryMaterializationState::Succeeded,
        )
    }

    fn verify(
        &self,
        document: &SublibraryMaterializationDocument,
    ) -> Result<SublibraryMaterialization, ApplicationError> {
        let sublibrary_id = &document.definition.id;
        let version = document.definition.version;
        let manifest = self.read_manifest(sublibrary_id, version)?;
        let materialization_bytes = serde_json::to_vec_pretty(document)
            .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
        let expected_sha256 = Sha256::of_bytes(&materialization_bytes);
        let expected_manifest = Self::manifest(document, &expected_sha256)?;
        if manifest != expected_manifest {
            return Err(ApplicationError::Integrity(
                "sublibrary manifest no longer matches the authoritative definition and projection"
                    .to_owned(),
            ));
        }
        let directory = self.ensure_safe_existing_directory(sublibrary_id, version)?;
        let actual =
            Sha256::of_bytes(&fs::read(directory.join("materialization.json")).map_err(io_error)?);
        if actual != expected_sha256 {
            return Err(ApplicationError::Integrity(
                "sublibrary materialization no longer matches its manifest hash".to_owned(),
            ));
        }
        Self::outcome(manifest, SublibraryMaterializationState::Verified)
    }

    fn delete(
        &self,
        sublibrary_id: &SublibraryId,
        version: u32,
    ) -> Result<SublibraryMaterialization, ApplicationError> {
        let manifest = self.read_manifest(sublibrary_id, version)?;
        let directory = self.ensure_safe_existing_directory(sublibrary_id, version)?;
        fs::remove_dir_all(directory).map_err(io_error)?;
        let parent = self.root().join(sublibrary_id.as_str());
        if parent.exists() && parent.read_dir().map_err(io_error)?.next().is_none() {
            fs::remove_dir(parent).map_err(io_error)?;
        }
        Self::outcome(manifest, SublibraryMaterializationState::Deleted)
    }
}

fn replace_file(path: PathBuf, bytes: &[u8]) -> Result<(), ApplicationError> {
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, bytes).map_err(io_error)?;
    if path.exists() {
        fs::remove_file(&path).map_err(io_error)?;
    }
    fs::rename(temporary, path).map_err(io_error)
}

fn io_error(error: io::Error) -> ApplicationError {
    ApplicationError::Storage(format!(
        "C2 sublibrary filesystem {:?} failure",
        error.kind()
    ))
}
