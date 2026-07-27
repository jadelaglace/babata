use std::{path::PathBuf, process::Command};

use babata_application::ApplicationError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct ResticConfig {
    pub executable: PathBuf,
    pub repository: PathBuf,
    pub password_file: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResticBackupSummary {
    pub snapshot_id: String,
    pub files_new: u64,
    pub files_changed: u64,
    pub files_unmodified: u64,
    pub data_added: u64,
}

impl ResticConfig {
    pub fn initialized(&self) -> bool {
        self.repository.join("config").is_file()
    }

    pub fn available(&self) -> Result<(), ApplicationError> {
        if !self.password_file.is_file() {
            return Err(ApplicationError::capability_unavailable(
                "ops.backup.password",
                "P8.1",
            ));
        }
        let output = Command::new(&self.executable)
            .arg("version")
            .output()
            .map_err(|error| ApplicationError::Storage(error.to_string()))?;
        if !output.status.success() {
            return Err(ApplicationError::capability_unavailable(
                "ops.backup.restic",
                "P8.1",
            ));
        }
        Ok(())
    }

    pub fn ensure_initialized(&self) -> Result<(), ApplicationError> {
        self.available()?;
        if self.initialized() {
            return Ok(());
        }
        std::fs::create_dir_all(&self.repository)
            .map_err(|error| ApplicationError::Storage(error.to_string()))?;
        self.run(["init"].as_slice()).map(|_| ())
    }

    pub fn check(&self) -> Result<(), ApplicationError> {
        self.run(["check"].as_slice()).map(|_| ())
    }

    pub fn backup(
        &self,
        working_directory: &std::path::Path,
        source_name: &str,
    ) -> Result<ResticBackupSummary, ApplicationError> {
        let output = self.run_in(
            ["backup", source_name, "--json", "--tag", "babata-p8-1"].as_slice(),
            Some(working_directory),
        )?;
        output
            .lines()
            .rev()
            .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
            .find(|value| {
                value.get("message_type").and_then(|value| value.as_str()) == Some("summary")
            })
            .map(|value| ResticBackupSummary {
                snapshot_id: value
                    .get("snapshot_id")
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    .to_owned(),
                files_new: json_u64(&value, "files_new"),
                files_changed: json_u64(&value, "files_changed"),
                files_unmodified: json_u64(&value, "files_unmodified"),
                data_added: json_u64(&value, "data_added"),
            })
            .filter(|summary| !summary.snapshot_id.is_empty())
            .ok_or_else(|| {
                ApplicationError::Integrity(
                    "restic backup completed without a JSON snapshot summary".to_owned(),
                )
            })
    }

    pub fn restore(
        &self,
        restic_snapshot_id: &str,
        target: &std::path::Path,
    ) -> Result<(), ApplicationError> {
        std::fs::create_dir_all(target)
            .map_err(|error| ApplicationError::Storage(error.to_string()))?;
        let target = target.to_string_lossy().into_owned();
        self.run(["restore", restic_snapshot_id, "--target", &target].as_slice())
            .map(|_| ())
    }

    fn run(&self, arguments: &[&str]) -> Result<String, ApplicationError> {
        self.run_in(arguments, None)
    }

    fn run_in(
        &self,
        arguments: &[&str],
        working_directory: Option<&std::path::Path>,
    ) -> Result<String, ApplicationError> {
        let mut command = Command::new(&self.executable);
        command
            .arg("--repo")
            .arg(&self.repository)
            .env("RESTIC_PASSWORD_FILE", &self.password_file)
            .args(arguments);
        if let Some(working_directory) = working_directory {
            command.current_dir(working_directory);
        }
        let output = command
            .output()
            .map_err(|error| ApplicationError::Storage(error.to_string()))?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
            return Err(ApplicationError::Storage(format!(
                "restic command failed with {}: {stderr}",
                output.status
            )));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

fn json_u64(value: &serde_json::Value, field: &str) -> u64 {
    value
        .get(field)
        .and_then(serde_json::Value::as_u64)
        .unwrap_or_default()
}
