use std::{
    fs,
    io::{self, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use babata_application::{
    ApplicationError,
    ports::{AssetStorePort, FinalizeAssetOutcome, StagedAsset},
};
use babata_domain::{AssetId, AssetIntegrityMethod, AssetRole, LogicalPath, Metadata, Sha256};
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

    fn modified_unix_nanos(metadata: &fs::Metadata) -> Option<u128> {
        metadata
            .modified()
            .ok()?
            .duration_since(UNIX_EPOCH)
            .ok()
            .map(|duration| duration.as_nanos())
    }

    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    fn stage_streaming(
        &self,
        source: &str,
        role: AssetRole,
        operation_id: &str,
        integrity_method: AssetIntegrityMethod,
        selected_relative_path: Option<&str>,
        expected_byte_size: Option<u64>,
        expected_modified_unix_nanos: Option<u128>,
    ) -> Result<StagedAsset, ApplicationError> {
        let source = Path::new(source);
        let pre = fs::metadata(source).map_err(Self::io)?;
        if !pre.is_file() {
            return Err(ApplicationError::Asset(
                "input must be a regular file".to_owned(),
            ));
        }
        let pre_modified = Self::modified_unix_nanos(&pre);
        if expected_byte_size.is_some_and(|expected| expected != pre.len())
            || expected_modified_unix_nanos.is_some_and(|expected| Some(expected) != pre_modified)
        {
            return Err(ApplicationError::Conflict(
                "source file changed after batch inventory".to_owned(),
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
                "integrity_method": integrity_method,
            }),
        )?;
        let asset_id = AssetId::new();
        let destination = staging_dir.join(format!("{asset_id}-{file_name}"));
        let copied = match integrity_method {
            AssetIntegrityMethod::SizeSnapshotV1 => fs::copy(source, &destination)
                .map(|copied| (copied, None))
                .map_err(Self::io),
            AssetIntegrityMethod::Sha256V1 => (|| {
                let mut reader = BufReader::new(fs::File::open(source).map_err(Self::io)?);
                let output = fs::File::create(&destination).map_err(Self::io)?;
                let mut writer = BufWriter::new(output);
                let mut hasher = Hasher::new();
                let mut copied = 0_u64;
                let mut buffer = vec![0_u8; 128 * 1024];
                loop {
                    let count = reader.read(&mut buffer).map_err(Self::io)?;
                    if count == 0 {
                        break;
                    }
                    writer.write_all(&buffer[..count]).map_err(Self::io)?;
                    hasher.update(&buffer[..count]);
                    copied = copied.checked_add(count as u64).ok_or_else(|| {
                        ApplicationError::Integrity("asset size overflow".to_owned())
                    })?;
                }
                writer.flush().map_err(Self::io)?;
                let sha256 = Sha256::parse(format!("{:x}", hasher.finalize()))
                    .map_err(ApplicationError::from)?;
                Ok::<_, ApplicationError>((copied, Some(sha256)))
            })(),
        };
        let (copied_byte_count, sha256) = match copied {
            Ok(result) => result,
            Err(error) => {
                if destination.exists() {
                    let _ = fs::remove_file(&destination);
                }
                let _ = self.remove_empty_staging(operation_id);
                let _ = self.complete_operation(operation_id);
                return Err(error);
            }
        };
        #[cfg(test)]
        if operation_id == "op_mutate_during_copy" {
            fs::OpenOptions::new()
                .append(true)
                .open(source)
                .and_then(|mut file| file.write_all(b"!"))
                .map_err(Self::io)?;
        }
        let post = fs::metadata(source).map_err(Self::io)?;
        let post_modified = Self::modified_unix_nanos(&post);
        if copied_byte_count != pre.len()
            || post.len() != pre.len()
            || post_modified != pre_modified
        {
            let _ = fs::remove_file(&destination);
            let _ = self.remove_empty_staging(operation_id);
            let _ = self.complete_operation(operation_id);
            return Err(ApplicationError::Conflict(
                "source file changed while it was being copied".to_owned(),
            ));
        }
        let logical_path = match &sha256 {
            Some(sha256) => LogicalPath::parse(format!(
                "01_raw/assets/sha256/{}/{sha256}",
                &sha256.as_str()[..2]
            )),
            None => LogicalPath::parse(format!("01_raw/assets/opaque/{asset_id}")),
        }
        .map_err(ApplicationError::from)?;
        let integrity_metadata = Metadata::parse(
            &serde_json::json!({
                "method": integrity_method,
                "selected_relative_path": selected_relative_path,
                "source": {
                    "pre": {
                        "byte_size": pre.len(),
                        "modified_unix_nanos": pre_modified.map(|value| value.to_string()),
                    },
                    "post": {
                        "byte_size": post.len(),
                        "modified_unix_nanos": post_modified.map(|value| value.to_string()),
                    }
                },
                "copied_byte_count": copied_byte_count,
            })
            .to_string(),
        )
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
            integrity_method,
            integrity_metadata,
            byte_size: copied_byte_count,
            media_type: mime_guess::from_path(source).first_raw().map(str::to_owned),
            original_filename: Some(file_name.to_owned()),
        })
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
        self.stage_streaming(
            source,
            role,
            operation_id,
            AssetIntegrityMethod::Sha256V1,
            None,
            None,
            None,
        )
    }

    fn stage_with_integrity(
        &self,
        source: &str,
        role: AssetRole,
        operation_id: &str,
        integrity_method: AssetIntegrityMethod,
        selected_relative_path: Option<&str>,
        expected_byte_size: Option<u64>,
        expected_modified_unix_nanos: Option<u128>,
    ) -> Result<StagedAsset, ApplicationError> {
        self.stage_streaming(
            source,
            role,
            operation_id,
            integrity_method,
            selected_relative_path,
            expected_byte_size,
            expected_modified_unix_nanos,
        )
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
            match &asset.sha256 {
                Some(expected) => {
                    let existing = fs::read(&final_path).map_err(Self::io)?;
                    let hash = Sha256::parse(format!("{:x}", Hasher::digest(existing)))
                        .map_err(ApplicationError::from)?;
                    if &hash != expected {
                        return Err(ApplicationError::Integrity(
                            "asset hash collision".to_owned(),
                        ));
                    }
                }
                None => {
                    if fs::metadata(&final_path).map_err(Self::io)?.len() != asset.byte_size {
                        return Err(ApplicationError::Integrity(
                            "opaque asset address contains a different size snapshot".to_owned(),
                        ));
                    }
                }
            }
            if staged.exists() {
                fs::remove_file(staged).map_err(Self::io)?;
            }
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
        match &asset.sha256 {
            Some(expected) => {
                let bytes = fs::read(path).map_err(Self::io)?;
                Ok(&Sha256::of_bytes(&bytes) == expected)
            }
            None => Ok(fs::metadata(path).map_err(Self::io)?.len() == asset.byte_size),
        }
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
        let sha256 = staged
            .sha256
            .as_ref()
            .expect("normal staging always produces sha256")
            .clone();
        staged.logical_path = LogicalPath::parse(format!(
            "02_derived/files/sha256/{}/{}",
            &sha256.as_str()[..2],
            sha256
        ))
        .map_err(ApplicationError::from)?;
        if let Err(error) = self.write_journal(
            operation_id,
            &serde_json::json!({
                "operation_id": operation_id,
                "state": "c1_staged",
                "logical_path": staged.logical_path.as_str(),
                "sha256": sha256.as_str(),
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
            "sha256": asset.sha256.as_ref().map(Sha256::as_str),
            "integrity_method": asset.integrity_method,
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
        assert_eq!(asset.sha256, Some(Sha256::of_bytes(content.as_bytes())));
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
        let sha256 = staged.sha256.as_ref().unwrap();
        assert_eq!(journal["sha256"], sha256.as_str());
        assert_eq!(
            staged.logical_path.as_str(),
            format!(
                "02_derived/files/sha256/{}/{}",
                &sha256.as_str()[..2],
                sha256
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
    fn size_snapshot_stages_to_opaque_address_without_sha256() {
        let temporary = tempdir().unwrap();
        let paths = DataPaths::new(temporary.path().to_path_buf());
        crate::paths::ensure_layout(&paths).unwrap();
        let input = temporary.path().join("unique.bin");
        std::fs::write(&input, b"unique-size-bytes").unwrap();
        let metadata = std::fs::metadata(&input).unwrap();
        let modified = FileAssetStore::modified_unix_nanos(&metadata);
        let store = FileAssetStore::new(paths);

        let staged = store
            .stage_with_integrity(
                &input.to_string_lossy(),
                AssetRole::Original,
                "op_opaque",
                AssetIntegrityMethod::SizeSnapshotV1,
                Some("unique.bin"),
                Some(metadata.len()),
                modified,
            )
            .unwrap();

        assert_eq!(staged.sha256, None);
        assert_eq!(
            staged.integrity_method,
            AssetIntegrityMethod::SizeSnapshotV1
        );
        assert!(
            staged
                .logical_path
                .as_str()
                .starts_with("01_raw/assets/opaque/asset_")
        );
        let integrity: serde_json::Value =
            serde_json::from_str(&staged.integrity_metadata.to_json()).unwrap();
        assert_eq!(integrity["copied_byte_count"], metadata.len());
        assert_eq!(integrity["selected_relative_path"], "unique.bin");
        store.finalize(&staged).unwrap();
        assert!(store.verify(&staged).unwrap());
    }

    #[test]
    fn inventory_snapshot_change_is_rejected_before_copy() {
        let temporary = tempdir().unwrap();
        let paths = DataPaths::new(temporary.path().to_path_buf());
        crate::paths::ensure_layout(&paths).unwrap();
        let input = temporary.path().join("changed.bin");
        std::fs::write(&input, b"changed").unwrap();
        let store = FileAssetStore::new(paths);
        let result = store.stage_with_integrity(
            &input.to_string_lossy(),
            AssetRole::Original,
            "op_changed",
            AssetIntegrityMethod::SizeSnapshotV1,
            Some("changed.bin"),
            Some(1),
            None,
        );
        assert!(matches!(result, Err(ApplicationError::Conflict(_))));
    }

    #[test]
    fn source_mutation_during_copy_is_rejected() {
        let temporary = tempdir().unwrap();
        let paths = DataPaths::new(temporary.path().to_path_buf());
        crate::paths::ensure_layout(&paths).unwrap();
        let input = temporary.path().join("mutating.bin");
        std::fs::write(&input, vec![b'x'; 256 * 1024]).unwrap();
        let metadata = std::fs::metadata(&input).unwrap();
        let store = FileAssetStore::new(paths);
        let result = store.stage_with_integrity(
            &input.to_string_lossy(),
            AssetRole::Original,
            "op_mutate_during_copy",
            AssetIntegrityMethod::SizeSnapshotV1,
            Some("mutating.bin"),
            Some(metadata.len()),
            FileAssetStore::modified_unix_nanos(&metadata),
        );
        assert!(matches!(result, Err(ApplicationError::Conflict(_))));
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
        let prefix = paths
            .raw_assets()
            .join(&asset.sha256.as_ref().unwrap().as_str()[..2]);
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
