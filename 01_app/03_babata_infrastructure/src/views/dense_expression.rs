use std::{fs, io, path::PathBuf};

use babata_application::{
    ApplicationError, DenseExpressionPreviewDocument, DenseExpressionPreviewOutcome,
    ports::DenseExpressionPreviewPort,
};
use babata_domain::{SemanticId, Sha256};
use serde::{Deserialize, Serialize};

use crate::paths::{DataPaths, ensure_layout};

const SCHEMA_VERSION: &str = "p6-dense-preview/v1";

#[derive(Clone)]
pub struct DenseExpressionViewStore {
    paths: DataPaths,
}

impl DenseExpressionViewStore {
    pub fn new(paths: DataPaths) -> Self {
        Self { paths }
    }

    fn root(&self) -> PathBuf {
        self.paths.root().join("03_views/p6_dense")
    }

    fn directory(&self, semantic_id: &str) -> Result<PathBuf, ApplicationError> {
        SemanticId::parse(semantic_id).map_err(ApplicationError::from)?;
        Ok(self.root().join(semantic_id))
    }

    fn ensure_safe_existing_directory(
        &self,
        semantic_id: &str,
    ) -> Result<PathBuf, ApplicationError> {
        let directory = self.directory(semantic_id)?;
        if !directory.exists() {
            return Err(ApplicationError::NotFound(format!(
                "dense expression preview for {semantic_id}"
            )));
        }
        let canonical_root = self.root().canonicalize().map_err(io_error)?;
        let canonical_directory = directory.canonicalize().map_err(io_error)?;
        if canonical_directory.parent() != Some(canonical_root.as_path()) {
            return Err(ApplicationError::Integrity(
                "dense expression preview escaped its controlled C2 root".to_owned(),
            ));
        }
        Ok(directory)
    }

    fn read_manifest(&self, semantic_id: &str) -> Result<PreviewManifest, ApplicationError> {
        let directory = self.ensure_safe_existing_directory(semantic_id)?;
        let bytes = fs::read(directory.join("manifest.json")).map_err(io_error)?;
        let manifest: PreviewManifest = serde_json::from_slice(&bytes)
            .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
        if manifest.schema_version != SCHEMA_VERSION || manifest.semantic_id != semantic_id {
            return Err(ApplicationError::Integrity(
                "dense expression preview manifest identity is invalid".to_owned(),
            ));
        }
        Ok(manifest)
    }

    fn outcome(
        semantic_id: &str,
        manifest: PreviewManifest,
        status: &str,
    ) -> Result<DenseExpressionPreviewOutcome, ApplicationError> {
        Ok(DenseExpressionPreviewOutcome {
            semantic_id: semantic_id.to_owned(),
            logical_path: format!("03_views/p6_dense/{semantic_id}/preview.md"),
            source_sha256: Sha256::parse(manifest.source_sha256).map_err(ApplicationError::from)?,
            output_sha256: Sha256::parse(manifest.output_sha256).map_err(ApplicationError::from)?,
            status: status.to_owned(),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct PreviewManifest {
    schema_version: String,
    semantic_id: String,
    source_sha256: String,
    output_sha256: String,
    files: Vec<String>,
}

impl DenseExpressionPreviewPort for DenseExpressionViewStore {
    fn build(
        &self,
        document: &DenseExpressionPreviewDocument,
    ) -> Result<DenseExpressionPreviewOutcome, ApplicationError> {
        ensure_layout(&self.paths).map_err(io_error)?;
        fs::create_dir_all(self.root()).map_err(io_error)?;
        let directory = self.directory(&document.semantic_id)?;
        let existed = directory.exists();
        if existed {
            self.ensure_safe_existing_directory(&document.semantic_id)?;
        } else {
            fs::create_dir(&directory).map_err(io_error)?;
        }
        let output_sha256 = Sha256::of_bytes(document.markdown.as_bytes());
        let manifest = PreviewManifest {
            schema_version: SCHEMA_VERSION.to_owned(),
            semantic_id: document.semantic_id.clone(),
            source_sha256: document.source_sha256.to_string(),
            output_sha256: output_sha256.to_string(),
            files: vec!["preview.md".to_owned()],
        };
        replace_file(directory.join("preview.md"), document.markdown.as_bytes())?;
        let manifest_json = serde_json::to_vec_pretty(&manifest)
            .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
        replace_file(directory.join("manifest.json"), &manifest_json)?;
        Self::outcome(
            &document.semantic_id,
            manifest,
            if existed { "rebuilt" } else { "built" },
        )
    }

    fn verify(
        &self,
        semantic_id: &str,
        source_sha256: &Sha256,
    ) -> Result<DenseExpressionPreviewOutcome, ApplicationError> {
        let manifest = self.read_manifest(semantic_id)?;
        if manifest.source_sha256 != source_sha256.as_str() {
            return Err(ApplicationError::Conflict(
                "dense expression preview is stale against the core text".to_owned(),
            ));
        }
        if manifest.files != ["preview.md"] {
            return Err(ApplicationError::Integrity(
                "dense expression preview manifest has unexpected files".to_owned(),
            ));
        }
        let directory = self.ensure_safe_existing_directory(semantic_id)?;
        let actual = Sha256::of_bytes(&fs::read(directory.join("preview.md")).map_err(io_error)?);
        if manifest.output_sha256 != actual.as_str() {
            return Err(ApplicationError::Integrity(
                "dense expression preview no longer matches its manifest hash".to_owned(),
            ));
        }
        Self::outcome(semantic_id, manifest, "verified")
    }

    fn delete(&self, semantic_id: &str) -> Result<DenseExpressionPreviewOutcome, ApplicationError> {
        let manifest = self.read_manifest(semantic_id)?;
        let directory = self.ensure_safe_existing_directory(semantic_id)?;
        fs::remove_dir_all(directory).map_err(io_error)?;
        Self::outcome(semantic_id, manifest, "deleted")
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
    ApplicationError::Storage(format!("C2 preview filesystem {:?} failure", error.kind()))
}
