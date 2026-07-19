use std::{
    fs, io,
    path::{Path, PathBuf},
};

use babata_application::{
    ApplicationError,
    ports::{AssetStorePort, FinalizeAssetOutcome, StagedAsset},
};
use babata_domain::{AssetId, AssetRole, LogicalPath, Sha256};
use sha2::{Digest, Sha256 as Hasher};

use crate::paths::DataPaths;

#[derive(Clone)]
pub struct FileAssetStore {
    paths: DataPaths,
}

impl FileAssetStore {
    pub fn new(paths: DataPaths) -> Self {
        Self { paths }
    }
    fn staged_path(&self, key: &str) -> PathBuf {
        self.paths.root().join(key)
    }
    fn operation_id(&self, asset: &StagedAsset) -> Result<String, ApplicationError> {
        self.staged_path(&asset.staging_key)
            .parent()
            .and_then(|path| path.file_name())
            .and_then(|name| name.to_str())
            .map(str::to_owned)
            .ok_or_else(|| {
                ApplicationError::Asset("staging operation identifier is invalid".to_owned())
            })
    }
    fn io(error: io::Error) -> ApplicationError {
        ApplicationError::Asset(format!("filesystem {:?} failure", error.kind()))
    }
    #[cfg(feature = "test-support")]
    fn fault(point: &str) -> bool {
        std::env::var("BABATA_TEST_ASSET_FAULT").is_ok_and(|value| value == point)
    }
    #[cfg(not(feature = "test-support"))]
    fn fault(_: &str) -> bool {
        false
    }
    fn remove_empty_staging(&self, operation_id: &str) -> Result<(), ApplicationError> {
        let staging_dir = self.paths.staging(operation_id);
        if staging_dir.exists()
            && fs::read_dir(&staging_dir)
                .map_err(Self::io)?
                .next()
                .is_none()
        {
            fs::remove_dir(&staging_dir).map_err(Self::io)?;
        }
        Ok(())
    }
    fn has_orphan(&self, operation_id: &str) -> Result<bool, ApplicationError> {
        let prefix = format!("{operation_id}-");
        Ok(fs::read_dir(self.paths.orphan())
            .map_err(Self::io)?
            .any(|entry| {
                entry
                    .ok()
                    .is_some_and(|entry| entry.file_name().to_string_lossy().starts_with(&prefix))
            }))
    }
    fn write_journal(
        &self,
        operation_id: &str,
        body: &serde_json::Value,
    ) -> Result<(), ApplicationError> {
        fs::write(
            self.paths.journal().join(format!("{operation_id}.json")),
            body.to_string(),
        )
        .map_err(Self::io)
    }

    pub fn open(&self, logical_path: &LogicalPath) -> Result<fs::File, ApplicationError> {
        fs::File::open(self.paths.resolve_logical(logical_path).map_err(Self::io)?)
            .map_err(Self::io)
    }
}

impl AssetStorePort for FileAssetStore {
    fn begin_operation(&self, operation_id: &str) -> Result<(), ApplicationError> {
        if self
            .paths
            .journal()
            .join(format!("{operation_id}.json"))
            .exists()
        {
            return Ok(());
        }
        self.write_journal(
            operation_id,
            &serde_json::json!({
                "operation_id": operation_id,
                "state": "allocated",
            }),
        )
    }

    fn preserve_operation(
        &self,
        operation_id: &str,
        revision_id: &str,
        failure_code: &str,
    ) -> Result<(), ApplicationError> {
        self.write_journal(
            operation_id,
            &serde_json::json!({
                "operation_id": operation_id,
                "revision_id": revision_id,
                "state": "recovery_required",
                "failure_code": failure_code,
            }),
        )
    }

    fn complete_operation(&self, operation_id: &str) -> Result<(), ApplicationError> {
        if Self::fault("cleanup") {
            return Err(ApplicationError::Asset(
                "injected cleanup failure".to_owned(),
            ));
        }
        self.remove_empty_staging(operation_id)?;
        if self.paths.staging(operation_id).exists() {
            return Err(ApplicationError::Asset(
                "operation staging is not empty".to_owned(),
            ));
        }
        if self.has_orphan(operation_id)? {
            return Err(ApplicationError::Asset(
                "operation has unresolved recovery markers".to_owned(),
            ));
        }
        let journal = self.paths.journal().join(format!("{operation_id}.json"));
        if journal.exists() {
            let body: serde_json::Value =
                serde_json::from_slice(&fs::read(&journal).map_err(Self::io)?).map_err(|_| {
                    ApplicationError::Asset("operation journal is invalid".to_owned())
                })?;
            if body.get("state").and_then(serde_json::Value::as_str) == Some("recovery_required") {
                return Err(ApplicationError::Asset(
                    "operation requires recovery".to_owned(),
                ));
            }
            fs::remove_file(journal).map_err(Self::io)?;
        }
        Ok(())
    }

    fn stage(
        &self,
        source: &str,
        role: AssetRole,
        operation_id: &str,
    ) -> Result<StagedAsset, ApplicationError> {
        let source = Path::new(source);
        let metadata = fs::metadata(source).map_err(Self::io)?;
        if !metadata.is_file() {
            return Err(ApplicationError::Asset(
                "input must be a regular file".to_owned(),
            ));
        }
        let file_name = source
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| ApplicationError::Asset("input filename is invalid".to_owned()))?;
        self.begin_operation(operation_id)?;
        let staging_dir = self.paths.staging(operation_id);
        fs::create_dir_all(&staging_dir).map_err(Self::io)?;
        self.write_journal(
            operation_id,
            &serde_json::json!({
                "operation_id": operation_id,
                "state": "staged",
            }),
        )?;
        let asset_id = AssetId::new();
        let destination = staging_dir.join(format!("{asset_id}-{file_name}"));
        let staged = (|| {
            fs::copy(source, &destination).map_err(Self::io)?;
            let bytes = fs::read(&destination).map_err(Self::io)?;
            let sha256 = Sha256::parse(format!("{:x}", Hasher::digest(&bytes)))
                .map_err(ApplicationError::from)?;
            Ok::<_, ApplicationError>((sha256, bytes.len() as u64))
        })();
        let (sha256, staged_size) = match staged {
            Ok(staged) => staged,
            Err(error) => {
                if destination.exists() {
                    let _ = fs::remove_file(&destination);
                }
                let _ = self.remove_empty_staging(operation_id);
                let _ = self.complete_operation(operation_id);
                return Err(error);
            }
        };
        let logical_path = LogicalPath::parse(format!(
            "01_raw/assets/sha256/{}/{sha256}",
            &sha256.as_str()[..2]
        ))
        .map_err(ApplicationError::from)?;
        Ok(StagedAsset {
            asset_id,
            role,
            staging_key: destination
                .strip_prefix(self.paths.root())
                .map_err(|_| ApplicationError::Asset("staging escaped data root".to_owned()))?
                .to_string_lossy()
                .replace('\\', "/"),
            logical_path,
            sha256,
            byte_size: staged_size,
            media_type: mime_guess::from_path(source).first_raw().map(str::to_owned),
            original_filename: Some(file_name.to_owned()),
        })
    }

    fn hash(&self, source: &str) -> Result<Sha256, ApplicationError> {
        let bytes = fs::read(source).map_err(Self::io)?;
        Sha256::parse(format!("{:x}", Hasher::digest(bytes))).map_err(ApplicationError::from)
    }

    fn finalize(&self, asset: &StagedAsset) -> Result<FinalizeAssetOutcome, ApplicationError> {
        if Self::fault("finalize") {
            return Err(ApplicationError::Asset(
                "injected finalization failure".to_owned(),
            ));
        }
        let staged = self.staged_path(&asset.staging_key);
        let final_path = self
            .paths
            .resolve_logical(&asset.logical_path)
            .map_err(Self::io)?;
        if let Some(parent) = final_path.parent() {
            fs::create_dir_all(parent).map_err(Self::io)?;
        }
        if final_path.exists() {
            let existing = fs::read(&final_path).map_err(Self::io)?;
            let hash = Sha256::parse(format!("{:x}", Hasher::digest(existing)))
                .map_err(ApplicationError::from)?;
            if hash != asset.sha256 {
                return Err(ApplicationError::Integrity(
                    "asset hash collision".to_owned(),
                ));
            }
            fs::remove_file(staged).map_err(Self::io)?;
            return Ok(FinalizeAssetOutcome::Reused);
        }
        fs::rename(staged, final_path).map_err(Self::io)?;
        Ok(FinalizeAssetOutcome::Created)
    }

    fn open(&self, logical_path: &LogicalPath) -> Result<Vec<u8>, ApplicationError> {
        let path = self.paths.resolve_logical(logical_path).map_err(Self::io)?;
        fs::read(path).map_err(Self::io)
    }

    fn verify(&self, asset: &StagedAsset) -> Result<bool, ApplicationError> {
        if Self::fault("verify") {
            return Ok(false);
        }
        let path = self
            .paths
            .resolve_logical(&asset.logical_path)
            .map_err(Self::io)?;
        if !path.exists() {
            return Ok(false);
        }
        let bytes = fs::read(path).map_err(Self::io)?;
        Ok(Sha256::of_bytes(&bytes) == asset.sha256)
    }

    fn discard_stage(&self, asset: &StagedAsset) -> Result<(), ApplicationError> {
        let path = self.staged_path(&asset.staging_key);
        if path.exists() {
            fs::remove_file(path).map_err(Self::io)?;
        }
        let operation_id = self.operation_id(asset)?;
        self.remove_empty_staging(&operation_id)
    }

    fn hash_logical(&self, logical_path: &LogicalPath) -> Result<Sha256, ApplicationError> {
        let path = self.paths.resolve_logical(logical_path).map_err(Self::io)?;
        let bytes = fs::read(path).map_err(Self::io)?;
        Ok(Sha256::of_bytes(&bytes))
    }

    fn stage_derived_file(
        &self,
        source: &str,
        operation_id: &str,
    ) -> Result<StagedAsset, ApplicationError> {
        let mut staged = self.stage(source, AssetRole::Derived, operation_id)?;
        staged.logical_path = LogicalPath::parse(format!(
            "02_derived/files/sha256/{}/{}",
            &staged.sha256.as_str()[..2],
            staged.sha256
        ))
        .map_err(ApplicationError::from)?;
        if let Err(error) = self.write_journal(
            operation_id,
            &serde_json::json!({
                "operation_id": operation_id,
                "state": "c1_staged",
                "logical_path": staged.logical_path.as_str(),
                "sha256": staged.sha256.as_str(),
            }),
        ) {
            let _ = self.discard_stage(&staged);
            let _ = self.complete_operation(operation_id);
            return Err(error);
        }
        Ok(staged)
    }

    fn quarantine_finalized(
        &self,
        asset: &StagedAsset,
        operation_id: &str,
        outcome: FinalizeAssetOutcome,
    ) -> Result<(), ApplicationError> {
        let captured_operation = self.operation_id(asset)?;
        if captured_operation != operation_id {
            return Err(ApplicationError::Integrity(
                "asset recovery operation does not match its staging journal".to_owned(),
            ));
        }
        let marker = self
            .paths
            .orphan()
            .join(format!("{captured_operation}-{}.json", asset.asset_id));
        let body = serde_json::json!({
            "operation_id": captured_operation,
            "state": "finalized_uncommitted",
            "asset_id": asset.asset_id.to_string(),
            "logical_path": asset.logical_path.as_str(),
            "sha256": asset.sha256.as_str(),
            "finalization": match outcome {
                FinalizeAssetOutcome::Created => "created",
                FinalizeAssetOutcome::Reused => "reused",
            },
        });
        fs::write(marker, body.to_string()).map_err(Self::io)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use babata_application::ports::AssetStorePort;
    use std::io::Read;
    use tempfile::tempdir;

    #[test]
    fn staged_asset_finalizes_with_matching_hash() {
        let temporary = tempdir().unwrap();
        let paths = DataPaths::new(temporary.path().to_path_buf());
        crate::paths::ensure_layout(&paths).unwrap();
        let input = temporary.path().join("fixture.txt");
        std::fs::write(&input, "fixture bytes").unwrap();
        let store = FileAssetStore::new(paths);
        let asset = store
            .stage(&input.to_string_lossy(), AssetRole::Original, "op_test")
            .unwrap();
        assert_eq!(
            store.finalize(&asset).unwrap(),
            FinalizeAssetOutcome::Created
        );
        let mut content = String::new();
        store
            .open(&asset.logical_path)
            .unwrap()
            .read_to_string(&mut content)
            .unwrap();
        assert_eq!(content, "fixture bytes");
        assert_eq!(asset.sha256, Sha256::of_bytes(content.as_bytes()));
    }

    #[test]
    fn staged_derivative_journal_records_final_content_addressed_path() {
        let temporary = tempdir().unwrap();
        let paths = DataPaths::new(temporary.path().to_path_buf());
        crate::paths::ensure_layout(&paths).unwrap();
        let input = temporary.path().join("result.md");
        std::fs::write(&input, "derived bytes").unwrap();
        let store = FileAssetStore::new(paths.clone());

        let staged = store
            .stage_derived_file(&input.to_string_lossy(), "c1_test")
            .unwrap();
        let journal: serde_json::Value =
            serde_json::from_slice(&std::fs::read(paths.journal().join("c1_test.json")).unwrap())
                .unwrap();
        assert_eq!(journal["state"], "c1_staged");
        assert_eq!(journal["logical_path"], staged.logical_path.as_str());
        assert_eq!(journal["sha256"], staged.sha256.as_str());
        assert_eq!(
            staged.logical_path.as_str(),
            format!(
                "02_derived/files/sha256/{}/{}",
                &staged.sha256.as_str()[..2],
                staged.sha256
            )
        );

        store.discard_stage(&staged).unwrap();
        store.complete_operation("c1_test").unwrap();
    }

    #[test]
    fn equal_assets_reuse_immutable_final_bytes() {
        let temporary = tempdir().unwrap();
        let paths = DataPaths::new(temporary.path().to_path_buf());
        crate::paths::ensure_layout(&paths).unwrap();
        let input = temporary.path().join("fixture.txt");
        std::fs::write(&input, "same").unwrap();
        let store = FileAssetStore::new(paths);
        let first = store
            .stage(&input.to_string_lossy(), AssetRole::Original, "op_one")
            .unwrap();
        assert_eq!(
            store.finalize(&first).unwrap(),
            FinalizeAssetOutcome::Created
        );
        let second = store
            .stage(&input.to_string_lossy(), AssetRole::Original, "op_two")
            .unwrap();
        assert_eq!(
            store.finalize(&second).unwrap(),
            FinalizeAssetOutcome::Reused
        );
        assert_eq!(first.logical_path, second.logical_path);
        assert!(!store.staged_path(&second.staging_key).exists());
    }

    #[test]
    fn failed_finalization_preserves_staging_and_journal_for_recovery() {
        let temporary = tempdir().unwrap();
        let paths = DataPaths::new(temporary.path().to_path_buf());
        crate::paths::ensure_layout(&paths).unwrap();
        let input = temporary.path().join("fixture.txt");
        std::fs::write(&input, "recoverable bytes").unwrap();
        let store = FileAssetStore::new(paths.clone());
        let asset = store
            .stage(&input.to_string_lossy(), AssetRole::Original, "op_recover")
            .unwrap();
        let prefix = paths.raw_assets().join(&asset.sha256.as_str()[..2]);
        std::fs::write(&prefix, "block final directory").unwrap();
        assert!(store.finalize(&asset).is_err());
        assert!(store.staged_path(&asset.staging_key).exists());
        assert!(paths.journal().join("op_recover.json").exists());
    }

    #[test]
    fn recovery_marker_does_not_move_content_addressed_bytes() {
        let temporary = tempdir().unwrap();
        let paths = DataPaths::new(temporary.path().to_path_buf());
        crate::paths::ensure_layout(&paths).unwrap();
        let input = temporary.path().join("fixture.txt");
        std::fs::write(&input, "shared bytes").unwrap();
        let store = FileAssetStore::new(paths.clone());
        let asset = store
            .stage(&input.to_string_lossy(), AssetRole::Original, "op_orphan")
            .unwrap();
        store.finalize(&asset).unwrap();
        store
            .quarantine_finalized(&asset, "op_orphan", FinalizeAssetOutcome::Created)
            .unwrap();
        assert!(store.verify(&asset).unwrap());
        assert_eq!(std::fs::read_dir(paths.orphan()).unwrap().count(), 1);
        assert!(paths.journal().join("op_orphan.json").exists());
    }
}
