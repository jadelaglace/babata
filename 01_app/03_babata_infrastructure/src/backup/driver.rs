use std::{
    collections::BTreeSet,
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Instant,
};

use babata_application::{
    ApplicationError, BackupOutcome, OperationStatus, RestoreVerificationOutcome,
    ports::{BackupDriverPort, ClockPort},
};
use babata_domain::{
    BackupClass, HealthState, RestoreState, Sha256, SnapshotId, SnapshotRef, UtcTimestamp,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256 as Sha256Hasher};
use walkdir::WalkDir;

use crate::{AppConfig, SystemClock, paths::DataPaths, sqlite::raw_status};

use super::{
    manifest::{SnapshotManifest, SnapshotManifestEntry},
    restic::{ResticBackupSummary, ResticConfig},
    sqlite_snapshot::{snapshot_database, verify_database},
};

const SNAPSHOT_MANIFEST_SCHEMA_VERSION: u32 = 1;
const DATABASE_PATHS: [&str; 4] = [
    "01_raw/index/raw.sqlite",
    "02_derived/index/derived.sqlite",
    "03_views/search/index/search.sqlite",
    "04_runtime/index/runtime.sqlite",
];

#[derive(Debug, Clone)]
pub struct ResticBackupDriver {
    data_paths: DataPaths,
    busy_timeout_ms: u64,
    backup_home: PathBuf,
    recovery_home: PathBuf,
    restic: ResticConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BackupState {
    snapshot: SnapshotRef,
    restic: ResticBackupSummary,
    file_count: u64,
    byte_count: u64,
}

#[derive(Debug)]
struct SnapshotFiles {
    manifest_sha256: Sha256,
    file_count: u64,
    byte_count: u64,
}

#[derive(Debug, Default)]
struct VerificationSummary {
    verified_files: u64,
    verified_bytes: u64,
    databases_checked: u64,
    c1_rebuildable_missing: Vec<String>,
    c2_views_missing: Vec<String>,
    c3_runtime_missing: Vec<String>,
}

impl ResticBackupDriver {
    pub fn from_config(config: &AppConfig) -> Self {
        let data_root = config.paths().root().to_path_buf();
        let parent = data_root.parent().unwrap_or_else(|| Path::new("."));
        let backup_home = std::env::var_os("BABATA_BACKUP_HOME")
            .map_or_else(|| parent.join("BabataBackups"), PathBuf::from);
        let recovery_home = std::env::var_os("BABATA_RECOVERY_HOME")
            .map_or_else(|| parent.join("BabataRecovery"), PathBuf::from);
        let restic_executable = std::env::var_os("BABATA_RESTIC_EXE").map_or_else(
            || recovery_home.join("tools/restic/restic.exe"),
            PathBuf::from,
        );
        let repository = std::env::var_os("BABATA_RESTIC_REPOSITORY")
            .map_or_else(|| backup_home.join("restic-repository"), PathBuf::from);
        let password_file = std::env::var_os("BABATA_RESTIC_PASSWORD_FILE").map_or_else(
            || recovery_home.join("recovery/p8-1-backup/restic-password.txt"),
            PathBuf::from,
        );
        Self {
            data_paths: config.paths(),
            busy_timeout_ms: config.sqlite.busy_timeout_ms,
            backup_home,
            recovery_home,
            restic: ResticConfig {
                executable: restic_executable,
                repository,
                password_file,
            },
        }
    }

    fn state_directory(&self) -> PathBuf {
        self.backup_home.join("state")
    }

    fn staging_directory(&self) -> PathBuf {
        self.backup_home.join("staging")
    }

    fn default_restore_target(&self, snapshot: &SnapshotId) -> PathBuf {
        self.recovery_home
            .join("recovery/p8-1-restores")
            .join(snapshot.to_string())
    }

    fn validate_roots(&self) -> Result<(), ApplicationError> {
        let data_root = self.data_paths.root();
        if self.backup_home.starts_with(data_root) || self.recovery_home.starts_with(data_root) {
            return Err(ApplicationError::Integrity(
                "backup and recovery roots must stay outside BABATA_DATA_HOME".to_owned(),
            ));
        }
        Ok(())
    }

    fn preflight(&self) -> Result<(), ApplicationError> {
        self.validate_roots()?;
        self.restic.available()?;
        let status = raw_status(&self.data_paths, self.busy_timeout_ms)?;
        if !status.reachable {
            return Err(ApplicationError::Integrity(
                "active raw database is not reachable".to_owned(),
            ));
        }
        let unsafe_count = status.pending_journals
            + status.orphans
            + status.quarantined_revisions
            + status.pending_operations
            + status.quarantined_operations
            + status.pending_asset_attachments
            + status.quarantined_asset_attachments;
        if unsafe_count != 0 {
            return Err(ApplicationError::Conflict(format!(
                "active data root has {unsafe_count} pending/quarantined/orphan recovery artifacts"
            )));
        }
        for relative_path in DATABASE_PATHS {
            let path = self.data_paths.root().join(relative_path);
            if path.is_file() {
                verify_database(&path)?;
            }
        }
        Ok(())
    }

    fn state_path(&self, snapshot: &SnapshotId) -> PathBuf {
        self.state_directory().join(format!("{snapshot}.json"))
    }

    fn write_state(&self, state: &BackupState) -> Result<(), ApplicationError> {
        std::fs::create_dir_all(self.state_directory())
            .map_err(|error| ApplicationError::Storage(error.to_string()))?;
        let bytes = serde_json::to_vec_pretty(state)
            .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
        std::fs::write(self.state_path(&state.snapshot.id), bytes)
            .map_err(|error| ApplicationError::Storage(error.to_string()))
    }

    fn read_state(&self, snapshot: &SnapshotId) -> Result<BackupState, ApplicationError> {
        let bytes = std::fs::read(self.state_path(snapshot))
            .map_err(|_| ApplicationError::NotFound(snapshot.to_string()))?;
        serde_json::from_slice(&bytes)
            .map_err(|error| ApplicationError::Integrity(error.to_string()))
    }

    fn latest_state(&self) -> Result<Option<BackupState>, ApplicationError> {
        if !self.state_directory().is_dir() {
            return Ok(None);
        }
        let mut paths = std::fs::read_dir(self.state_directory())
            .map_err(|error| ApplicationError::Storage(error.to_string()))?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "json")
            })
            .collect::<Vec<_>>();
        paths.sort();
        let Some(path) = paths.last() else {
            return Ok(None);
        };
        let bytes =
            std::fs::read(path).map_err(|error| ApplicationError::Storage(error.to_string()))?;
        serde_json::from_slice(&bytes)
            .map(Some)
            .map_err(|error| ApplicationError::Integrity(error.to_string()))
    }
}

impl BackupDriverPort for ResticBackupDriver {
    fn status(&self) -> Result<OperationStatus, ApplicationError> {
        self.validate_roots()?;
        self.restic.available()?;
        match self.latest_state()? {
            Some(state) => {
                if !self.restic.initialized() {
                    return Err(ApplicationError::Integrity(
                        "backup state exists but the restic repository is missing".to_owned(),
                    ));
                }
                Ok(OperationStatus {
                    health: HealthState::Healthy,
                    detail: format!(
                        "latest snapshot {} maps to restic {} ({} files, {} bytes)",
                        state.snapshot.id,
                        state.restic.snapshot_id,
                        state.file_count,
                        state.byte_count
                    ),
                })
            }
            None => Ok(OperationStatus {
                health: HealthState::Degraded,
                detail: "backup tooling is ready but no completed snapshot exists".to_owned(),
            }),
        }
    }

    fn doctor(&self) -> Result<OperationStatus, ApplicationError> {
        self.preflight()?;
        if self.restic.initialized() {
            self.restic.check()?;
        }
        Ok(OperationStatus {
            health: HealthState::Healthy,
            detail: "active databases are healthy, recovery state is empty, and encrypted backup tooling is ready".to_owned(),
        })
    }

    fn snapshot(&self) -> Result<BackupOutcome, ApplicationError> {
        let started = Instant::now();
        self.preflight()?;
        self.restic.ensure_initialized()?;
        std::fs::create_dir_all(self.staging_directory())
            .map_err(|error| ApplicationError::Storage(error.to_string()))?;

        let snapshot_id = SnapshotId::new();
        let created_at = SystemClock.now();
        let snapshot_root = self.staging_directory().join(snapshot_id.to_string());
        if snapshot_root.exists() {
            return Err(ApplicationError::Conflict(format!(
                "snapshot staging path already exists: {}",
                snapshot_root.display()
            )));
        }
        let files = create_snapshot_files(
            &self.data_paths,
            &snapshot_root,
            snapshot_id.clone(),
            created_at.clone(),
            self.busy_timeout_ms,
        )?;
        let snapshot = SnapshotRef {
            id: snapshot_id.clone(),
            created_at,
            manifest_sha256: files.manifest_sha256,
        };
        let restic = self
            .restic
            .backup(&self.staging_directory(), &snapshot_id.to_string())?;
        self.restic.check()?;
        let state = BackupState {
            snapshot: snapshot.clone(),
            restic: restic.clone(),
            file_count: files.file_count,
            byte_count: files.byte_count,
        };
        self.write_state(&state)?;
        remove_snapshot_staging(&snapshot_root, &self.staging_directory())?;
        Ok(BackupOutcome {
            snapshot,
            restic_snapshot_id: restic.snapshot_id,
            file_count: files.file_count,
            byte_count: files.byte_count,
            files_new: restic.files_new,
            files_changed: restic.files_changed,
            files_unmodified: restic.files_unmodified,
            data_added: restic.data_added,
            elapsed_ms: elapsed_ms(started),
        })
    }

    fn restore_verify(
        &self,
        snapshot: &SnapshotId,
        target: Option<&str>,
    ) -> Result<RestoreVerificationOutcome, ApplicationError> {
        let started = Instant::now();
        self.validate_roots()?;
        self.restic.available()?;
        let state = self.read_state(snapshot)?;
        let target = target.map_or_else(|| self.default_restore_target(snapshot), PathBuf::from);
        if target.starts_with(self.data_paths.root()) {
            return Err(ApplicationError::Integrity(
                "restore target must stay outside the active data root".to_owned(),
            ));
        }
        if target.exists()
            && std::fs::read_dir(&target)
                .map_err(|error| ApplicationError::Storage(error.to_string()))?
                .next()
                .is_some()
        {
            return Err(ApplicationError::Conflict(format!(
                "restore target is not empty: {}",
                target.display()
            )));
        }
        self.restic.restore(&state.restic.snapshot_id, &target)?;
        let snapshot_root = find_snapshot_root(&target, snapshot)?;
        verification_outcome(state, &snapshot_root, &target, started)
    }

    fn verify_restored(
        &self,
        snapshot: &SnapshotId,
        target: &str,
    ) -> Result<RestoreVerificationOutcome, ApplicationError> {
        let started = Instant::now();
        self.validate_roots()?;
        let state = self.read_state(snapshot)?;
        let target = PathBuf::from(target);
        if target.starts_with(self.data_paths.root()) {
            return Err(ApplicationError::Integrity(
                "verification target must stay outside the active data root".to_owned(),
            ));
        }
        let snapshot_root = find_snapshot_root(&target, snapshot)?;
        verification_outcome(state, &snapshot_root, &target, started)
    }
}

fn verification_outcome(
    state: BackupState,
    snapshot_root: &Path,
    target: &Path,
    started: Instant,
) -> Result<RestoreVerificationOutcome, ApplicationError> {
    let verification = verify_snapshot_root(snapshot_root, &state.snapshot)?;
    Ok(RestoreVerificationOutcome {
        snapshot: state.snapshot,
        restic_snapshot_id: state.restic.snapshot_id,
        state: RestoreState::Verified,
        target: target.to_string_lossy().into_owned(),
        verified_files: verification.verified_files,
        verified_bytes: verification.verified_bytes,
        databases_checked: verification.databases_checked,
        c1_rebuildable_missing: verification.c1_rebuildable_missing,
        c2_views_missing: verification.c2_views_missing,
        c3_runtime_missing: verification.c3_runtime_missing,
        credentials_reauthorization_required: true,
        elapsed_ms: elapsed_ms(started),
    })
}

fn create_snapshot_files(
    data_paths: &DataPaths,
    snapshot_root: &Path,
    snapshot_id: SnapshotId,
    created_at: UtcTimestamp,
    busy_timeout_ms: u64,
) -> Result<SnapshotFiles, ApplicationError> {
    let data_root = snapshot_root.join("data");
    std::fs::create_dir_all(&data_root)
        .map_err(|error| ApplicationError::Storage(error.to_string()))?;
    let mut entries = Vec::new();

    for entry in WalkDir::new(data_paths.root()).sort_by_file_name() {
        let entry = entry.map_err(|error| ApplicationError::Storage(error.to_string()))?;
        let path = entry.path();
        let relative = path
            .strip_prefix(data_paths.root())
            .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
        let relative_path = normalized_relative_path(relative)?;
        if relative_path.is_empty() || skip_transient(&relative_path) {
            continue;
        }
        if entry.file_type().is_symlink() {
            return Err(ApplicationError::Integrity(format!(
                "snapshot input contains a symlink: {relative_path}"
            )));
        }
        if !entry.file_type().is_file() || is_sqlite_sidecar(&relative_path) {
            continue;
        }
        let destination = data_root.join(relative);
        let (byte_size, sha256) = if DATABASE_PATHS.contains(&relative_path.as_str()) {
            snapshot_database(path, &destination, busy_timeout_ms)?;
            hash_file(&destination)?
        } else {
            copy_and_hash_stable(path, &destination)?
        };
        entries.push(SnapshotManifestEntry {
            relative_path,
            sha256,
            byte_size,
            class: backup_class(relative),
        });
    }

    entries.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    let file_count = u64::try_from(entries.len())
        .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
    let byte_count = entries.iter().map(|entry| entry.byte_size).sum();
    let manifest = SnapshotManifest {
        schema_version: SNAPSHOT_MANIFEST_SCHEMA_VERSION,
        snapshot_id,
        created_at,
        entries,
    };
    let manifest_bytes = serde_json::to_vec_pretty(&manifest)
        .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
    let manifest_sha256 = Sha256::of_bytes(&manifest_bytes);
    std::fs::write(snapshot_root.join("manifest.json"), manifest_bytes)
        .map_err(|error| ApplicationError::Storage(error.to_string()))?;
    Ok(SnapshotFiles {
        manifest_sha256,
        file_count,
        byte_count,
    })
}

fn verify_snapshot_root(
    snapshot_root: &Path,
    snapshot: &SnapshotRef,
) -> Result<VerificationSummary, ApplicationError> {
    let manifest_path = snapshot_root.join("manifest.json");
    let manifest_bytes = std::fs::read(&manifest_path)
        .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
    let manifest_sha256 = Sha256::of_bytes(&manifest_bytes);
    if manifest_sha256 != snapshot.manifest_sha256 {
        return Err(ApplicationError::Integrity(format!(
            "manifest hash mismatch: expected {}, got {}",
            snapshot.manifest_sha256, manifest_sha256
        )));
    }
    let manifest: SnapshotManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
    if manifest.snapshot_id != snapshot.id {
        return Err(ApplicationError::Integrity(
            "restored manifest references another snapshot".to_owned(),
        ));
    }
    let mut summary = VerificationSummary::default();
    let mut critical_failures = Vec::new();
    let data_root = snapshot_root.join("data");
    for entry in manifest.entries {
        let path = safe_join(&data_root, &entry.relative_path)?;
        let verification = hash_file(&path).and_then(|(byte_size, sha256)| {
            if byte_size == entry.byte_size && sha256 == entry.sha256 {
                Ok((byte_size, sha256))
            } else {
                Err(ApplicationError::Integrity(format!(
                    "hash or size mismatch for {}",
                    entry.relative_path
                )))
            }
        });
        match verification {
            Ok((byte_size, _)) => {
                summary.verified_files += 1;
                summary.verified_bytes += byte_size;
                if DATABASE_PATHS.contains(&entry.relative_path.as_str()) {
                    match verify_database(&path) {
                        Ok(()) => summary.databases_checked += 1,
                        Err(error) => record_class_failure(
                            entry.class,
                            format!("{}: {error}", entry.relative_path),
                            &mut critical_failures,
                            &mut summary,
                        ),
                    }
                }
            }
            Err(error) => record_class_failure(
                entry.class,
                format!("{}: {error}", entry.relative_path),
                &mut critical_failures,
                &mut summary,
            ),
        }
    }
    if !critical_failures.is_empty() {
        return Err(ApplicationError::Integrity(format!(
            "C0 restore verification failed: {}",
            critical_failures.join("; ")
        )));
    }
    Ok(summary)
}

fn record_class_failure(
    class: BackupClass,
    detail: String,
    critical_failures: &mut Vec<String>,
    summary: &mut VerificationSummary,
) {
    match class {
        BackupClass::C0Authority => critical_failures.push(detail),
        BackupClass::C1Derived => summary.c1_rebuildable_missing.push(detail),
        BackupClass::C2Views => summary.c2_views_missing.push(detail),
        BackupClass::C3Runtime => summary.c3_runtime_missing.push(detail),
    }
}

fn copy_and_hash_stable(
    source: &Path,
    destination: &Path,
) -> Result<(u64, Sha256), ApplicationError> {
    let before = source
        .metadata()
        .map_err(|error| ApplicationError::Storage(error.to_string()))?;
    if let Some(parent) = destination.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|error| ApplicationError::Storage(error.to_string()))?;
    }
    let mut input =
        File::open(source).map_err(|error| ApplicationError::Storage(error.to_string()))?;
    let mut output = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(destination)
        .map_err(|error| ApplicationError::Storage(error.to_string()))?;
    let mut hasher = Sha256Hasher::new();
    let mut buffer = vec![0_u8; 1024 * 1024];
    let mut byte_size = 0_u64;
    loop {
        let read = input
            .read(&mut buffer)
            .map_err(|error| ApplicationError::Storage(error.to_string()))?;
        if read == 0 {
            break;
        }
        output
            .write_all(&buffer[..read])
            .map_err(|error| ApplicationError::Storage(error.to_string()))?;
        hasher.update(&buffer[..read]);
        byte_size +=
            u64::try_from(read).map_err(|error| ApplicationError::Integrity(error.to_string()))?;
    }
    output
        .sync_all()
        .map_err(|error| ApplicationError::Storage(error.to_string()))?;
    let after = source
        .metadata()
        .map_err(|error| ApplicationError::Storage(error.to_string()))?;
    if before.len() != after.len()
        || before.modified().ok() != after.modified().ok()
        || byte_size != after.len()
    {
        return Err(ApplicationError::Conflict(format!(
            "source changed during snapshot: {}",
            source.display()
        )));
    }
    let sha256 = Sha256::parse(format!("{:x}", hasher.finalize()))?;
    Ok((byte_size, sha256))
}

fn hash_file(path: &Path) -> Result<(u64, Sha256), ApplicationError> {
    let mut file = File::open(path)
        .map_err(|error| ApplicationError::Integrity(format!("{}: {error}", path.display())))?;
    let mut hasher = Sha256Hasher::new();
    let mut buffer = vec![0_u8; 1024 * 1024];
    let mut byte_size = 0_u64;
    loop {
        let read = file
            .read(&mut buffer)
            .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        byte_size +=
            u64::try_from(read).map_err(|error| ApplicationError::Integrity(error.to_string()))?;
    }
    Ok((
        byte_size,
        Sha256::parse(format!("{:x}", hasher.finalize()))?,
    ))
}

fn normalized_relative_path(relative: &Path) -> Result<String, ApplicationError> {
    let mut parts = Vec::new();
    for component in relative.components() {
        match component {
            std::path::Component::Normal(value) => {
                parts.push(value.to_string_lossy().into_owned());
            }
            _ => {
                return Err(ApplicationError::Integrity(format!(
                    "snapshot relative path is unsafe: {}",
                    relative.display()
                )));
            }
        }
    }
    Ok(parts.join("/"))
}

fn safe_join(root: &Path, relative: &str) -> Result<PathBuf, ApplicationError> {
    let mut path = root.to_path_buf();
    for part in relative.split('/') {
        if part.is_empty() || part == "." || part == ".." || part.contains(['\\', ':']) {
            return Err(ApplicationError::Integrity(format!(
                "manifest path escaped restore root: {relative}"
            )));
        }
        path.push(part);
    }
    Ok(path)
}

fn backup_class(relative: &Path) -> BackupClass {
    match relative
        .components()
        .next()
        .and_then(|component| match component {
            std::path::Component::Normal(value) => value.to_str(),
            _ => None,
        }) {
        Some("00_inbox" | "01_raw") => BackupClass::C0Authority,
        Some("02_derived") => BackupClass::C1Derived,
        Some("03_views") => BackupClass::C2Views,
        _ => BackupClass::C3Runtime,
    }
}

fn skip_transient(relative: &str) -> bool {
    relative == "05_logs"
        || relative.starts_with("05_logs/")
        || relative == "04_runtime/staging"
        || relative.starts_with("04_runtime/staging/")
        || relative == "04_runtime/asset-journal"
        || relative.starts_with("04_runtime/asset-journal/")
}

fn is_sqlite_sidecar(relative: &str) -> bool {
    DATABASE_PATHS.iter().any(|database| {
        relative == format!("{database}-wal") || relative == format!("{database}-shm")
    })
}

fn find_snapshot_root(target: &Path, snapshot: &SnapshotId) -> Result<PathBuf, ApplicationError> {
    let expected = snapshot.to_string();
    let candidates = WalkDir::new(target)
        .max_depth(6)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file() && entry.file_name() == "manifest.json")
        .filter_map(|entry| entry.path().parent().map(Path::to_path_buf))
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name == expected.as_str())
        })
        .collect::<BTreeSet<_>>();
    match candidates.len() {
        1 => Ok(candidates.into_iter().next().expect("one candidate exists")),
        0 => Err(ApplicationError::Integrity(format!(
            "restored snapshot manifest was not found for {snapshot}"
        ))),
        count => Err(ApplicationError::Integrity(format!(
            "restored snapshot contains {count} matching manifests"
        ))),
    }
}

fn remove_snapshot_staging(path: &Path, staging_root: &Path) -> Result<(), ApplicationError> {
    if path.parent() != Some(staging_root) || path == staging_root {
        return Err(ApplicationError::Integrity(format!(
            "refusing to remove unsafe snapshot staging path: {}",
            path.display()
        )));
    }
    std::fs::remove_dir_all(path).map_err(|error| ApplicationError::Storage(error.to_string()))
}

fn elapsed_ms(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn consistent_snapshot_verifies_and_c0_corruption_fails_closed() {
        let source = tempdir().unwrap();
        let snapshot = tempdir().unwrap();
        let paths = DataPaths::new(source.path().to_path_buf());
        crate::paths::ensure_layout(&paths).unwrap();
        for database in DATABASE_PATHS {
            let path = source.path().join(database);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            let connection = rusqlite::Connection::open(path).unwrap();
            connection
                .execute_batch("CREATE TABLE example (id INTEGER PRIMARY KEY, value TEXT); INSERT INTO example(value) VALUES ('ok');")
                .unwrap();
        }
        let asset = paths.raw_assets().join("aa/fixture.bin");
        std::fs::create_dir_all(asset.parent().unwrap()).unwrap();
        std::fs::write(&asset, b"authority").unwrap();
        let snapshot_id = SnapshotId::new();
        let created_at = SystemClock.now();
        let files = create_snapshot_files(
            &paths,
            snapshot.path(),
            snapshot_id.clone(),
            created_at.clone(),
            100,
        )
        .unwrap();
        let reference = SnapshotRef {
            id: snapshot_id,
            created_at,
            manifest_sha256: files.manifest_sha256,
        };
        let verified = verify_snapshot_root(snapshot.path(), &reference).unwrap();
        assert_eq!(verified.databases_checked, 4);
        std::fs::write(
            snapshot
                .path()
                .join("data/01_raw/assets/sha256/aa/fixture.bin"),
            b"corrupt",
        )
        .unwrap();
        let error = verify_snapshot_root(snapshot.path(), &reference).unwrap_err();
        assert!(error.to_string().contains("C0 restore verification failed"));
    }

    #[test]
    fn manifest_paths_cannot_escape_restore_root() {
        let root = tempdir().unwrap();
        assert!(safe_join(root.path(), "../escape").is_err());
    }
}
