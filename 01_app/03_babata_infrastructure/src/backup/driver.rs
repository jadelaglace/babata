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
    manifest::{SnapshotExclusion, SnapshotManifest, SnapshotManifestEntry, SnapshotRoot},
    restic::{ResticBackupSummary, ResticConfig},
    roots::{is_link, overlaps, resolved_path, validate_roots},
    sqlite_snapshot::{snapshot_database, verify_database},
};

const SNAPSHOT_MANIFEST_SCHEMA_VERSION: u32 = 2;
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
    evidence_home: PathBuf,
    assistant_home: PathBuf,
    restic: ResticConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BackupState {
    #[serde(default)]
    repository: Option<PathBuf>,
    #[serde(default)]
    repository_id: Option<String>,
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
        let evidence_home = std::env::var_os("BABATA_EVIDENCE_HOME")
            .map_or_else(|| parent.join("BabataEvidence"), PathBuf::from);
        let assistant_home = std::env::var_os("BABATA_ASSISTANT_HOME")
            .map_or_else(|| parent.join("BabataAssistant"), PathBuf::from);
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
            evidence_home,
            assistant_home,
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
        let mut roots = self
            .source_roots()
            .into_iter()
            .map(|(_, path)| path)
            .collect::<Vec<_>>();
        roots.push(self.backup_home.clone());
        validate_roots(&roots)?;
        let repository = resolved_path(&self.restic.repository)?;
        validate_roots(std::slice::from_ref(&repository))?;
        for generated in [self.staging_directory(), self.state_directory()] {
            if overlaps(&repository, &resolved_path(&generated)?) {
                return Err(ApplicationError::Integrity(
                    "repository overlaps backup staging or state".to_owned(),
                ));
            }
        }
        for (_, source) in self.source_roots() {
            if overlaps(&repository, &resolved_path(&source)?) {
                return Err(ApplicationError::Integrity(
                    "repository overlaps a source root".to_owned(),
                ));
            }
        }
        Ok(())
    }

    fn source_roots(&self) -> Vec<(&str, PathBuf)> {
        vec![
            ("data", self.data_paths.root().to_path_buf()),
            ("evidence", self.evidence_home.clone()),
            ("recovery", self.recovery_home.clone()),
            ("assistant", self.assistant_home.clone()),
        ]
    }

    fn collection_exclusions() -> Result<Vec<SnapshotExclusion>, ApplicationError> {
        let Some(path) = std::env::var_os("BABATA_BACKUP_EXCLUSIONS_FILE") else {
            return Ok(Vec::new());
        };
        let bytes =
            std::fs::read(path).map_err(|error| ApplicationError::Storage(error.to_string()))?;
        serde_json::from_slice(&bytes)
            .map_err(|error| ApplicationError::Integrity(error.to_string()))
    }

    fn validate_state_location(&self, state: &BackupState) -> Result<(), ApplicationError> {
        let repository = state.repository.as_ref().ok_or_else(|| {
            ApplicationError::Integrity(
                "legacy backup state has unknown repository location".to_owned(),
            )
        })?;
        if resolved_path(repository)? != resolved_path(&self.restic.repository)? {
            return Err(ApplicationError::Integrity(
                "backup state belongs to another repository".to_owned(),
            ));
        }
        if state.repository_id.as_deref() != Some(self.restic.repository_id()?.as_str()) {
            return Err(ApplicationError::Integrity(
                "backup repository identity changed or is unknown".to_owned(),
            ));
        }
        self.restic.require_snapshot(&state.restic.snapshot_id)
    }

    fn validate_restore_target(&self, target: &Path) -> Result<(), ApplicationError> {
        let target = resolved_path(target)?;
        validate_roots(std::slice::from_ref(&target))?;
        for source in self
            .source_roots()
            .into_iter()
            .map(|(_, path)| path)
            .chain([self.backup_home.clone(), self.restic.repository.clone()])
        {
            // A fresh child of Recovery is allowed, but not its source contents.
            let root = resolved_path(&source)?;
            let recovery_child =
                source == self.recovery_home && target != root && target.starts_with(&root);
            if overlaps(&target, &root) && !recovery_child {
                return Err(ApplicationError::Integrity(
                    "restore target overlaps protected roots".to_owned(),
                ));
            }
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
                self.validate_state_location(&state)?;
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
            if let Some(state) = self.latest_state()? {
                self.validate_state_location(&state)?;
            }
        }
        Ok(OperationStatus {
            health: HealthState::Healthy,
            detail: "active databases are healthy, recovery state is empty, and encrypted backup tooling is ready".to_owned(),
        })
    }

    fn snapshot(&self) -> Result<BackupOutcome, ApplicationError> {
        let started = Instant::now();
        self.preflight()?;
        let exclusions = Self::collection_exclusions()?;
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
        let files = create_collection_files(
            &self.source_roots(),
            &snapshot_root,
            snapshot_id.clone(),
            created_at.clone(),
            self.busy_timeout_ms,
            &self.restic.password_file,
            &exclusions,
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
            repository: Some(resolved_path(&self.restic.repository)?),
            repository_id: Some(self.restic.repository_id()?),
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
        self.validate_state_location(&state)?;
        let target = target.map_or_else(|| self.default_restore_target(snapshot), PathBuf::from);
        self.validate_restore_target(&target)?;
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
        self.validate_restore_target(&target)?;
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

fn create_collection_files(
    roots: &[(&str, PathBuf)],
    snapshot_root: &Path,
    snapshot_id: SnapshotId,
    created_at: UtcTimestamp,
    busy_timeout_ms: u64,
    password_file: &Path,
    declared_exclusions: &[SnapshotExclusion],
) -> Result<SnapshotFiles, ApplicationError> {
    let mut entries = Vec::new();
    let mut exclusions = Vec::new();
    let mut root_manifest = Vec::new();
    let mut historical_groups = Vec::new();
    validate_exclusions(roots, snapshot_root, declared_exclusions)?;
    for (alias, source_root) in roots {
        if !source_root.is_dir() {
            return Err(ApplicationError::Integrity(format!(
                "required collection root is missing: {alias}"
            )));
        }
        root_manifest.push(SnapshotRoot {
            alias: (*alias).to_owned(),
            source: source_root.to_string_lossy().into_owned(),
        });
        let data_root = snapshot_root.join(alias);
        std::fs::create_dir_all(&data_root)
            .map_err(|error| ApplicationError::Storage(error.to_string()))?;
        let mut walker = WalkDir::new(source_root).sort_by_file_name().into_iter();

        while let Some(entry) = walker.next() {
            let entry = entry.map_err(|error| ApplicationError::Storage(error.to_string()))?;
            let path = entry.path();
            let relative = path
                .strip_prefix(source_root)
                .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
            let relative_path = normalized_relative_path(relative)?;
            if relative_path.is_empty() {
                continue;
            }
            if is_link(path)? {
                return Err(ApplicationError::Integrity(format!(
                    "snapshot input contains a symlink: {relative_path}"
                )));
            }
            if let Some(rule) = declared_exclusions
                .iter()
                .find(|rule| rule.root_alias == *alias && rule.relative_path == relative_path)
            {
                exclusions.push(rule.clone());
                if entry.file_type().is_dir() {
                    walker.skip_current_dir();
                }
                continue;
            }
            require_profile_rule(path)?;
            if let Some(reason) = credential_exclusion(path, password_file) {
                exclusions.push(SnapshotExclusion {
                    root_alias: (*alias).to_owned(),
                    relative_path,
                    reason: reason.to_owned(),
                });
                if entry.file_type().is_dir() {
                    walker.skip_current_dir();
                }
                continue;
            }
            if !entry.file_type().is_file() {
                continue;
            }
            if active_sidecar(alias, source_root, path)? {
                continue;
            }
            let destination = data_root.join(relative);
            let (byte_size, sha256) = if active_database(alias, &relative_path) {
                snapshot_database(path, &destination, busy_timeout_ms)?;
                hash_file(&destination)?
            } else {
                if is_sqlite_file(path)? {
                    historical_groups.push(sqlite_group_fingerprint(path)?);
                }
                copy_and_hash_stable(path, &destination)?
            };
            entries.push(SnapshotManifestEntry {
                root_alias: (*alias).to_owned(),
                relative_path,
                sha256,
                byte_size,
                class: if *alias == "data" {
                    backup_class(relative)
                } else {
                    BackupClass::C0Authority
                },
            });
        }
    }

    for group in historical_groups {
        if sqlite_group_fingerprint(&group.0)? != group {
            return Err(ApplicationError::Conflict(
                "historical SQLite group changed during snapshot".to_owned(),
            ));
        }
    }
    write_collection_manifest(
        snapshot_root,
        snapshot_id,
        created_at,
        root_manifest,
        exclusions,
        entries,
    )
}

fn validate_exclusions(
    roots: &[(&str, PathBuf)],
    snapshot_root: &Path,
    rules: &[SnapshotExclusion],
) -> Result<(), ApplicationError> {
    for rule in rules {
        if !roots.iter().any(|(alias, _)| *alias == rule.root_alias)
            || rule.reason.trim().is_empty()
        {
            return Err(ApplicationError::Integrity(
                "invalid declared exclusion".to_owned(),
            ));
        }
        safe_join(snapshot_root, &rule.relative_path)?;
    }
    Ok(())
}

fn write_collection_manifest(
    snapshot_root: &Path,
    snapshot_id: SnapshotId,
    created_at: UtcTimestamp,
    roots: Vec<SnapshotRoot>,
    exclusions: Vec<SnapshotExclusion>,
    mut entries: Vec<SnapshotManifestEntry>,
) -> Result<SnapshotFiles, ApplicationError> {
    entries.sort_by(|left, right| {
        (&left.root_alias, &left.relative_path).cmp(&(&right.root_alias, &right.relative_path))
    });
    let file_count = u64::try_from(entries.len())
        .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
    let byte_count = entries.iter().map(|entry| entry.byte_size).sum();
    let manifest = SnapshotManifest {
        schema_version: SNAPSHOT_MANIFEST_SCHEMA_VERSION,
        snapshot_id,
        created_at,
        finished_at: Some(SystemClock.now()),
        roots,
        exclusions,
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
    if !(1..=SNAPSHOT_MANIFEST_SCHEMA_VERSION).contains(&manifest.schema_version) {
        return Err(ApplicationError::Integrity(
            "unsupported snapshot manifest version".to_owned(),
        ));
    }
    for entry in manifest.entries {
        if !["data", "evidence", "recovery", "assistant"].contains(&entry.root_alias.as_str()) {
            return Err(ApplicationError::Integrity(
                "unknown snapshot root alias".to_owned(),
            ));
        }
        let data_root = snapshot_root.join(&entry.root_alias);
        let path = safe_join(&data_root, &entry.relative_path)?;
        // A restore tree supplied by the caller must not redirect verification.
        let mut cursor = path.as_path();
        while cursor != snapshot_root {
            if cursor.exists() && is_link(cursor)? {
                return Err(ApplicationError::Integrity(
                    "restored tree contains a link".to_owned(),
                ));
            }
            cursor = cursor.parent().ok_or_else(|| {
                ApplicationError::Integrity("restored path escaped collection".to_owned())
            })?;
        }
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
                if active_database(&entry.root_alias, &entry.relative_path) {
                    match verify_database(&path) {
                        Ok(()) => summary.databases_checked += 1,
                        Err(error) => record_class_failure(
                            BackupClass::C0Authority,
                            format!("{}: {error}", entry.relative_path),
                            &mut critical_failures,
                            &mut summary,
                        ),
                    }
                }
            }
            Err(error) => record_class_failure(
                if entry.root_alias == "data" {
                    entry.class
                } else {
                    BackupClass::C0Authority
                },
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
        Some("02_derived") => BackupClass::C1Derived,
        Some("03_views") => BackupClass::C2Views,
        // Unknown runtime/log content is not proven rebuildable.
        _ => BackupClass::C0Authority,
    }
}

fn credential_exclusion(path: &Path, password_file: &Path) -> Option<&'static str> {
    if path == password_file {
        return Some("configured backup key reference; managed separately");
    }
    let name = path.file_name()?.to_str()?.to_lowercase();
    if matches!(
        name.as_str(),
        ".env" | "credentials.json" | "restic-password.txt" | "id_rsa" | "id_ed25519"
    ) || name.starts_with(".env.")
    {
        return Some("credential file; reauthorization or separate secure protection required");
    }
    None
}

fn sqlite_sidecar_base(path: &Path) -> Option<PathBuf> {
    let name = path.to_str()?;
    let base = name
        .strip_suffix("-wal")
        .or_else(|| name.strip_suffix("-shm"))?;
    let base = PathBuf::from(base);
    is_sqlite_file(&base)
        .ok()
        .filter(|value| *value)
        .map(|_| base)
}

fn active_database(alias: &str, relative: &str) -> bool {
    alias == "data" && DATABASE_PATHS.contains(&relative)
}

fn active_sidecar(alias: &str, source_root: &Path, path: &Path) -> Result<bool, ApplicationError> {
    let Some(base) = sqlite_sidecar_base(path) else {
        return Ok(false);
    };
    let relative = base
        .strip_prefix(source_root)
        .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
    Ok(active_database(alias, &normalized_relative_path(relative)?))
}

fn require_profile_rule(path: &Path) -> Result<(), ApplicationError> {
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.to_lowercase().contains("browser-profile"))
    {
        return Err(ApplicationError::Integrity(
            "browser profile requires an explicit reviewed exclusion/protection rule".to_owned(),
        ));
    }
    Ok(())
}

type SqliteGroupFingerprint = (PathBuf, Vec<Option<(u64, std::time::SystemTime)>>);

fn sqlite_group_fingerprint(path: &Path) -> Result<SqliteGroupFingerprint, ApplicationError> {
    let mut files = Vec::new();
    for suffix in ["", "-wal", "-shm", "-journal"] {
        let mut name = path.as_os_str().to_os_string();
        name.push(suffix);
        let candidate = PathBuf::from(name);
        match std::fs::metadata(&candidate) {
            Ok(metadata) => files.push(Some((
                metadata.len(),
                metadata
                    .modified()
                    .map_err(|error| ApplicationError::Storage(error.to_string()))?,
            ))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => files.push(None),
            Err(error) => return Err(ApplicationError::Storage(error.to_string())),
        }
    }
    Ok((path.to_path_buf(), files))
}

fn is_sqlite_file(path: &Path) -> Result<bool, ApplicationError> {
    let mut file =
        File::open(path).map_err(|error| ApplicationError::Storage(error.to_string()))?;
    let mut header = [0_u8; 16];
    let count = file
        .read(&mut header)
        .map_err(|error| ApplicationError::Storage(error.to_string()))?;
    Ok(count == 16 && &header == b"SQLite format 3\0")
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
        let files = create_collection_files(
            &[("data", paths.root().to_path_buf())],
            snapshot.path(),
            snapshot_id.clone(),
            created_at.clone(),
            100,
            &source.path().join("unused-password"),
            &[],
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
        assert!(safe_join(root.path(), "C:/escape").is_err());
        let roots = [("data", root.path().to_path_buf())];
        assert!(
            validate_exclusions(
                &roots,
                root.path(),
                &[SnapshotExclusion {
                    root_alias: "data".to_owned(),
                    relative_path: "../escape".to_owned(),
                    reason: "test".to_owned()
                }]
            )
            .is_err()
        );
        assert!(
            validate_exclusions(
                &roots,
                root.path(),
                &[SnapshotExclusion {
                    root_alias: "data".to_owned(),
                    relative_path: "safe".to_owned(),
                    reason: String::new()
                }]
            )
            .is_err()
        );
    }

    #[test]
    fn collection_preserves_unknown_runtime_and_auxiliary_files_and_explains_exclusions() {
        let fixture = tempdir().unwrap();
        let output = tempdir().unwrap();
        let roots = ["data", "evidence", "recovery", "assistant"].map(|alias| {
            let root = fixture.path().join(alias);
            std::fs::create_dir(&root).unwrap();
            (alias, root)
        });
        let paid = roots[0]
            .1
            .join("04_runtime/staging/model-workspaces/legacy/paid.txt");
        std::fs::create_dir_all(paid.parent().unwrap()).unwrap();
        std::fs::write(&paid, b"paid unique result").unwrap();
        let receipt = roots[1].1.join("receipt.json");
        std::fs::write(&receipt, b"immutable historical evidence").unwrap();
        std::fs::write(roots[2].1.join("unknown.bin"), b"unknown unique input").unwrap();
        std::fs::write(roots[3].1.join("local-script.ps1"), b"unique script").unwrap();
        std::fs::write(roots[2].1.join("restic-password.txt"), b"fixture secret").unwrap();
        let profile = roots[3].1.join("arena-browser-profile");
        std::fs::create_dir(&profile).unwrap();
        std::fs::write(profile.join("Cookies"), b"fixture login").unwrap();
        let db = roots[3].1.join("workspace.sqlite");
        let connection = rusqlite::Connection::open(&db).unwrap();
        connection.execute_batch("PRAGMA journal_mode=WAL; CREATE TABLE example (value TEXT); INSERT INTO example VALUES ('wal committed');").unwrap();
        let source_bytes = std::fs::read(&db).unwrap();
        let source_wal = std::fs::read(roots[3].1.join("workspace.sqlite-wal")).unwrap();
        let id = SnapshotId::new();
        let now = SystemClock.now();
        let files = create_collection_files(
            &roots,
            output.path(),
            id.clone(),
            now.clone(),
            100,
            &roots[2].1.join("restic-password.txt"),
            &[SnapshotExclusion {
                root_alias: "assistant".to_owned(),
                relative_path: "arena-browser-profile".to_owned(),
                reason: "fixture login profile; no unique user work".to_owned(),
            }],
        )
        .unwrap();
        assert_eq!(files.file_count, 7);
        assert_eq!(
            std::fs::read(output.path().join("assistant/workspace.sqlite")).unwrap(),
            source_bytes
        );
        assert_eq!(
            std::fs::read(output.path().join("assistant/workspace.sqlite-wal")).unwrap(),
            source_wal
        );
        assert!(
            output
                .path()
                .join("data/04_runtime/staging/model-workspaces/legacy/paid.txt")
                .is_file()
        );
        assert!(!output.path().join("recovery/restic-password.txt").exists());
        assert!(
            !output
                .path()
                .join("assistant/arena-browser-profile")
                .exists()
        );
        assert_eq!(
            std::fs::read(&receipt).unwrap(),
            b"immutable historical evidence"
        );
        let manifest: SnapshotManifest =
            serde_json::from_slice(&std::fs::read(output.path().join("manifest.json")).unwrap())
                .unwrap();
        assert_eq!(manifest.roots.len(), 4);
        assert_eq!(manifest.exclusions.len(), 2);
        assert!(manifest.finished_at.is_some());
        let reference = SnapshotRef {
            id,
            created_at: now,
            manifest_sha256: files.manifest_sha256,
        };
        assert_eq!(
            verify_snapshot_root(output.path(), &reference)
                .unwrap()
                .databases_checked,
            0
        );
        let restored =
            rusqlite::Connection::open(output.path().join("assistant/workspace.sqlite")).unwrap();
        let value: String = restored
            .query_row("SELECT value FROM example", [], |row| row.get(0))
            .unwrap();
        assert_eq!(value, "wal committed");
        std::fs::remove_file(output.path().join("evidence/receipt.json")).unwrap();
        assert!(verify_snapshot_root(output.path(), &reference).is_err());
    }

    #[test]
    fn historical_sqlite_bytes_are_not_rewritten_or_health_reinterpreted() {
        let source = tempdir().unwrap();
        let output = tempdir().unwrap();
        let db = source.path().join("damaged-test-evidence.sqlite");
        let bytes = [
            b"SQLite format 3\0".as_slice(),
            b"intentionally invalid historical fixture",
        ]
        .concat();
        std::fs::write(&db, &bytes).unwrap();
        let id = SnapshotId::new();
        let now = SystemClock.now();
        let result = create_collection_files(
            &[("evidence", source.path().to_path_buf())],
            output.path(),
            id.clone(),
            now.clone(),
            100,
            &source.path().join("key"),
            &[],
        )
        .unwrap();
        assert_eq!(
            std::fs::read(output.path().join("evidence/damaged-test-evidence.sqlite")).unwrap(),
            bytes
        );
        let reference = SnapshotRef {
            id,
            created_at: now,
            manifest_sha256: result.manifest_sha256,
        };
        assert_eq!(
            verify_snapshot_root(output.path(), &reference)
                .unwrap()
                .databases_checked,
            0
        );
        let before = sqlite_group_fingerprint(&db).unwrap();
        std::fs::write(
            source.path().join("damaged-test-evidence.sqlite-wal"),
            b"drift",
        )
        .unwrap();
        assert_ne!(sqlite_group_fingerprint(&db).unwrap(), before);
    }

    #[test]
    fn missing_collection_root_and_unknown_manifest_alias_fail_closed() {
        let fixture = tempdir().unwrap();
        let output = tempdir().unwrap();
        assert!(
            create_collection_files(
                &[("evidence", fixture.path().join("missing"))],
                output.path(),
                SnapshotId::new(),
                SystemClock.now(),
                100,
                &fixture.path().join("key"),
                &[]
            )
            .is_err()
        );
        let id = SnapshotId::new();
        let now = SystemClock.now();
        let manifest = SnapshotManifest {
            schema_version: 2,
            snapshot_id: id.clone(),
            created_at: now.clone(),
            roots: vec![],
            exclusions: vec![],
            finished_at: Some(now.clone()),
            entries: vec![SnapshotManifestEntry {
                root_alias: "../escape".to_owned(),
                relative_path: "file".to_owned(),
                sha256: Sha256::of_bytes(b"a"),
                byte_size: 1,
                class: BackupClass::C0Authority,
            }],
        };
        let bytes = serde_json::to_vec(&manifest).unwrap();
        std::fs::write(output.path().join("manifest.json"), &bytes).unwrap();
        let reference = SnapshotRef {
            id,
            created_at: now,
            manifest_sha256: Sha256::of_bytes(&bytes),
        };
        assert!(
            verify_snapshot_root(output.path(), &reference)
                .unwrap_err()
                .to_string()
                .contains("root alias")
        );
    }

    fn test_driver(root: &Path) -> ResticBackupDriver {
        ResticBackupDriver {
            data_paths: DataPaths::new(root.join("data")),
            busy_timeout_ms: 100,
            backup_home: root.join("backup"),
            recovery_home: root.join("recovery"),
            evidence_home: root.join("evidence"),
            assistant_home: root.join("assistant"),
            restic: ResticConfig {
                executable: root.join("not-executed"),
                repository: root.join("backup/repository"),
                password_file: root.join("key"),
            },
        }
    }

    #[test]
    fn restore_targets_cannot_replace_any_source_or_repository() {
        let fixture = tempdir().unwrap();
        let driver = test_driver(fixture.path());
        for target in [
            "data",
            "evidence/new",
            "assistant/new",
            "recovery",
            "backup/new",
        ] {
            assert!(
                driver
                    .validate_restore_target(&fixture.path().join(target))
                    .is_err(),
                "{target}"
            );
        }
        assert!(
            driver
                .validate_restore_target(&fixture.path().join("recovery/new-empty-restore"))
                .is_ok()
        );
        assert!(driver.validate_restore_target(fixture.path()).is_err());
    }

    #[test]
    fn repository_cannot_overlap_staging_or_state() {
        let fixture = tempdir().unwrap();
        let mut driver = test_driver(fixture.path());
        for relative in [
            "backup",
            "backup/staging",
            "backup/staging/repository",
            "backup/state/repository",
        ] {
            driver.restic.repository = fixture.path().join(relative);
            assert!(driver.validate_roots().is_err(), "{relative}");
        }
        driver.restic.repository = fixture.path().join("backup/repository");
        assert!(driver.validate_roots().is_ok());
    }

    #[test]
    fn real_restic_four_root_roundtrip_when_explicitly_configured() {
        let Some(executable) = std::env::var_os("BABATA_TEST_RESTIC_EXE") else {
            return;
        };
        let fixture = tempdir().unwrap();
        let mut driver = test_driver(fixture.path());
        driver.restic.executable = executable.into();
        std::fs::write(
            &driver.restic.password_file,
            b"fixture-only-not-a-real-password",
        )
        .unwrap();
        for (_, root) in driver.source_roots() {
            std::fs::create_dir_all(root).unwrap();
        }
        crate::paths::ensure_layout(&driver.data_paths).unwrap();
        drop(crate::open_raw_database(&driver.data_paths, 100).unwrap());
        std::fs::write(driver.evidence_home.join("receipt.json"), b"evidence").unwrap();
        std::fs::write(driver.assistant_home.join("unique.txt"), b"assistant").unwrap();
        std::fs::write(driver.recovery_home.join("unique.txt"), b"recovery").unwrap();
        driver.restic.ensure_initialized().unwrap();
        assert!(driver.restic.require_snapshot(&"f".repeat(64)).is_err());
        let outcome = driver.snapshot().unwrap();
        assert!(matches!(
            driver.status().unwrap().health,
            HealthState::Healthy
        ));
        let restored = driver.restore_verify(&outcome.snapshot.id, None).unwrap();
        assert!(matches!(restored.state, RestoreState::Verified));
        assert!(restored.verified_files >= 4);
        assert!(driver.restore_verify(&outcome.snapshot.id, None).is_err());
        let target = PathBuf::from(restored.target);
        let snapshot_root = find_snapshot_root(&target, &outcome.snapshot.id).unwrap();
        std::fs::write(snapshot_root.join("assistant/unique.txt"), b"corrupt").unwrap();
        assert!(
            driver
                .verify_restored(&outcome.snapshot.id, &target.to_string_lossy())
                .is_err()
        );
        let mut state = driver.read_state(&outcome.snapshot.id).unwrap();
        state.repository_id = Some("wrong-repository".to_owned());
        assert!(driver.validate_state_location(&state).is_err());
    }

    #[test]
    fn legacy_or_wrong_repository_state_is_not_healthy() {
        let fixture = tempdir().unwrap();
        let driver = test_driver(fixture.path());
        let mut state = BackupState {
            repository: None,
            repository_id: None,
            snapshot: SnapshotRef {
                id: SnapshotId::new(),
                created_at: SystemClock.now(),
                manifest_sha256: Sha256::of_bytes(b"manifest"),
            },
            restic: ResticBackupSummary {
                snapshot_id: "a".repeat(64),
                files_new: 0,
                files_changed: 0,
                files_unmodified: 0,
                data_added: 0,
            },
            file_count: 0,
            byte_count: 0,
        };
        assert!(
            driver
                .validate_state_location(&state)
                .unwrap_err()
                .to_string()
                .contains("unknown repository")
        );
        state.repository = Some(fixture.path().join("elsewhere"));
        assert!(
            driver
                .validate_state_location(&state)
                .unwrap_err()
                .to_string()
                .contains("another repository")
        );
    }

    #[test]
    fn legacy_v1_manifest_still_verifies_without_rewriting_history() {
        let fixture = tempdir().unwrap();
        let id = SnapshotId::new();
        let now = SystemClock.now();
        let path = fixture.path().join("data/01_raw/original.txt");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"original").unwrap();
        let bytes = serde_json::to_vec(&serde_json::json!({
            "schema_version":1,"snapshot_id":id,"created_at":now,
            "entries":[{"relative_path":"01_raw/original.txt","sha256":Sha256::of_bytes(b"original"),"byte_size":8,"class":"c0_authority"}]
        })).unwrap();
        std::fs::write(fixture.path().join("manifest.json"), &bytes).unwrap();
        let reference = SnapshotRef {
            id,
            created_at: now,
            manifest_sha256: Sha256::of_bytes(&bytes),
        };
        assert_eq!(
            verify_snapshot_root(fixture.path(), &reference)
                .unwrap()
                .verified_files,
            1
        );
    }
}
