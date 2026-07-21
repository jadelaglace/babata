use assert_cmd::Command;
use serde_json::Value;
use tempfile::tempdir;

fn babata(temp: &tempfile::TempDir) -> Command {
    let mut command = Command::cargo_bin("babata").unwrap();
    command.env("BABATA_DATA_HOME", temp.path());
    command
}

fn capture_text(temp: &tempfile::TempDir, text: &str) -> Value {
    let output = babata(temp)
        .args([
            "--json",
            "capture",
            "text",
            "--provider",
            "fixture",
            "--text",
            text,
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&output).unwrap()
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest;
    format!("{:x}", sha2::Sha256::digest(bytes))
}

fn capture_file(temp: &tempfile::TempDir, path: &std::path::Path) -> Value {
    let output = babata(temp)
        .args([
            "--json",
            "capture",
            "file",
            "--provider",
            "fixture",
            "--path",
            &path.to_string_lossy(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&output).unwrap()
}

fn file_count(path: &std::path::Path) -> usize {
    if !path.exists() {
        return 0;
    }
    std::fs::read_dir(path)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .map(|path| if path.is_dir() { file_count(&path) } else { 1 })
        .sum()
}

fn fake_bailian_cli(temp: &tempfile::TempDir) -> std::path::PathBuf {
    #[cfg(windows)]
    {
        let path = temp.path().join("fake-bl.cmd");
        std::fs::write(
            &path,
            r#"@echo off
if "%1"=="--version" (
  echo bl 1.10.0
  exit /b 0
)
if "%1"=="auth" (
  echo {"authenticated":true}
  exit /b 0
)
if "%BABATA_TEST_PROVIDER_FAIL%"=="1" (
  echo intentional provider failure 1>&2
  exit /b 7
)
echo {"choices":[{"message":{"content":"queue summary"}}],"usage":{"input_tokens":3,"output_tokens":2},"request_id":"fake-request"}
"#,
        )
        .unwrap();
        path
    }
    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt;
        let path = temp.path().join("fake-bl");
        std::fs::write(
            &path,
            r#"#!/bin/sh
if [ "$1" = "--version" ]; then echo 'bl 1.10.0'; exit 0; fi
if [ "$1" = "auth" ]; then echo '{"authenticated":true}'; exit 0; fi
if [ "$BABATA_TEST_PROVIDER_FAIL" = "1" ]; then echo 'intentional provider failure' >&2; exit 7; fi
echo '{"choices":[{"message":{"content":"queue summary"}}],"usage":{"input_tokens":3,"output_tokens":2},"request_id":"fake-request"}'
"#,
        )
        .unwrap();
        let mut permissions = std::fs::metadata(&path).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&path, permissions).unwrap();
        path
    }
}

#[test]
fn capture_text_emits_a_parseable_json_envelope() {
    let temp = tempdir().unwrap();
    let output = babata(&temp)
        .args([
            "--json",
            "capture",
            "text",
            "--provider",
            "fixture",
            "--text",
            "hello",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let value: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(value["status"], "ready");
    assert!(value["item_id"].as_str().unwrap().starts_with("item_"));
}

#[test]
fn invalid_text_emits_a_json_error_envelope() {
    let temp = tempdir().unwrap();
    let output = babata(&temp)
        .args([
            "--json",
            "capture",
            "text",
            "--provider",
            "fixture",
            "--text",
            " ",
        ])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let value: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(value["code"], "validation_failed");
    assert_eq!(value["retryable"], false);
}

#[test]
fn attach_assets_appends_original_and_preview_without_mutating_parent() {
    let temp = tempdir().unwrap();
    let capture = capture_text(&temp, "real document body retained from the source");
    let parent_revision = capture["revision_id"].as_str().unwrap();
    let item_id = capture["item_id"].as_str().unwrap();
    let parent_hash = capture["record"]["revisions"][0]["text_sha256"]
        .as_str()
        .unwrap();
    let original = temp.path().join("source.docx");
    let preview = temp.path().join("platform-preview.pdf");
    std::fs::write(&original, b"original-docx-bytes").unwrap();
    std::fs::write(&preview, b"preview-pdf-bytes").unwrap();

    let output = babata(&temp)
        .args([
            "--json",
            "capture",
            "attach-assets",
            "--revision",
            parent_revision,
            "--original",
            original.to_str().unwrap(),
            "--preview",
            preview.to_str().unwrap(),
            "--reason",
            "recover source file and distinguish platform preview",
            "--metadata-json",
            r#"{"recovery":"fixture"}"#,
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let output: Value = serde_json::from_slice(&output).unwrap();

    assert_eq!(output["item_id"], item_id);
    assert_eq!(output["revision_id"], parent_revision);
    assert_eq!(output["reimported"], false);
    assert_eq!(output["record"]["revisions"].as_array().unwrap().len(), 1);
    assert_eq!(output["record"]["revisions"][0]["text_sha256"], parent_hash);
    let assets = output["record"]["assets"].as_array().unwrap();
    assert_eq!(assets.len(), 2);
    assert!(assets.iter().any(|asset| {
        asset["role"] == "original" && asset["sha256"] == sha256_hex(b"original-docx-bytes")
    }));
    assert!(assets.iter().any(|asset| {
        asset["role"] == "preview" && asset["sha256"] == sha256_hex(b"preview-pdf-bytes")
    }));
    for asset in assets {
        assert_eq!(asset["revision_id"], parent_revision);
        assert_eq!(asset["state"], "ready");
        assert!(
            temp.path()
                .join(asset["logical_path"].as_str().unwrap())
                .exists()
        );
    }
    let attachments = output["record"]["asset_attachments"].as_array().unwrap();
    assert_eq!(attachments.len(), 1);
    assert_eq!(attachments[0]["revision_id"], parent_revision);
    assert_eq!(
        attachments[0]["reason"],
        "recover source file and distinguish platform preview"
    );
    assert_eq!(attachments[0]["metadata"]["recovery"], "fixture");
    assert_eq!(attachments[0]["state"], "ready");
    assert_eq!(attachments[0]["asset_ids"].as_array().unwrap().len(), 2);
}

#[test]
fn capability_list_reports_processing_enabled_for_p5_register() {
    let temp = tempdir().unwrap();
    let output = babata(&temp)
        .env(
            "BABATA_BAILIAN_CLI",
            temp.path().join("missing-bailian-cli"),
        )
        .args(["--json", "capabilities", "list"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let value: Value = serde_json::from_slice(&output).unwrap();
    assert!(value.as_array().unwrap().iter().any(|descriptor| {
        descriptor["id"] == "processing" && descriptor["status"] == "enabled"
    }));
    assert!(value.as_array().unwrap().iter().any(|descriptor| {
        descriptor["id"] == "processing.local_extract" && descriptor["status"] == "enabled"
    }));
    for provider in ["processing.bailian_cli", "processing.bailian_api"] {
        assert!(value.as_array().unwrap().iter().any(|descriptor| {
            descriptor["id"] == provider && descriptor["status"] == "unavailable"
        }));
    }
}

#[test]
fn process_list_pipelines_is_enabled() {
    let temp = tempdir().unwrap();
    let output = babata(&temp)
        .args(["--json", "process", "list-pipelines"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let value: Value = serde_json::from_slice(&output).unwrap();
    assert!(
        value
            .as_array()
            .unwrap()
            .iter()
            .any(|pipeline| pipeline == "agent_import")
    );
}

#[test]
fn process_queue_local_extract_runs_from_a_real_c0_text_asset() {
    let temp = tempdir().unwrap();
    let source = temp.path().join("queue-input.txt");
    let source_bytes = b"exact queue input\nsecond line";
    std::fs::write(&source, source_bytes).unwrap();
    let capture = capture_file(&temp, &source);
    let revision = capture["revision_id"].as_str().unwrap();
    let asset = &capture["record"]["assets"][0];
    let original_asset = temp.path().join(asset["logical_path"].as_str().unwrap());

    let queued: Value = serde_json::from_slice(
        &babata(&temp)
            .args([
                "--json",
                "process",
                "enqueue",
                "--pipeline",
                "local_extract_text",
                "--revision",
                revision,
            ])
            .assert()
            .success()
            .get_output()
            .stdout,
    )
    .unwrap();
    assert_eq!(queued["status"], "queued");
    assert_eq!(queued["job"]["input_asset_id"], asset["asset_id"]);
    assert_eq!(queued["job"]["input_sha256"], sha256_hex(source_bytes));

    let completed: Value = serde_json::from_slice(
        &babata(&temp)
            .args(["--json", "process", "run-once"])
            .assert()
            .success()
            .get_output()
            .stdout,
    )
    .unwrap();
    assert_eq!(completed["status"], "succeeded");
    let run_id = completed["job"]["result_run_id"].as_str().unwrap();
    let shown: Value = serde_json::from_slice(
        &babata(&temp)
            .args(["--json", "process", "show-run", "--run", run_id])
            .assert()
            .success()
            .get_output()
            .stdout,
    )
    .unwrap();
    assert_eq!(shown["run"]["provider"], "local_extract");
    assert_eq!(shown["run"]["input_asset_id"], asset["asset_id"]);
    assert_eq!(
        shown["derivatives"][0]["content_text"],
        "exact queue input\nsecond line"
    );
    assert_eq!(std::fs::read(original_asset).unwrap(), source_bytes);
}

#[test]
fn process_queue_cancel_and_unavailable_paths_are_explicit() {
    let temp = tempdir().unwrap();
    let source = temp.path().join("cancel-input.txt");
    std::fs::write(&source, "cancel me").unwrap();
    let capture = capture_file(&temp, &source);
    let revision = capture["revision_id"].as_str().unwrap();
    let queued: Value = serde_json::from_slice(
        &babata(&temp)
            .args([
                "--json",
                "process",
                "enqueue",
                "--pipeline",
                "local_extract_text",
                "--revision",
                revision,
            ])
            .assert()
            .success()
            .get_output()
            .stdout,
    )
    .unwrap();
    let job = queued["job"]["id"].as_str().unwrap();
    let cancelled: Value = serde_json::from_slice(
        &babata(&temp)
            .args(["--json", "process", "cancel", job])
            .assert()
            .success()
            .get_output()
            .stdout,
    )
    .unwrap();
    assert_eq!(cancelled["status"], "cancelled");
    let idle: Value = serde_json::from_slice(
        &babata(&temp)
            .args(["--json", "process", "run-once"])
            .assert()
            .success()
            .get_output()
            .stdout,
    )
    .unwrap();
    assert_eq!(idle["status"], "idle");

    let unavailable: Value = serde_json::from_slice(
        &babata(&temp)
            .args([
                "--json",
                "process",
                "enqueue",
                "--pipeline",
                "bailian_ocr",
                "--revision",
                revision,
            ])
            .assert()
            .failure()
            .get_output()
            .stdout,
    )
    .unwrap();
    assert_eq!(unavailable["code"], "capability_unavailable");
}

#[test]
fn process_queue_provider_failure_and_retry_preserve_both_attempts() {
    let temp = tempdir().unwrap();
    let fake_bl = fake_bailian_cli(&temp);
    let capture = capture_text(&temp, "source text that remains unchanged across retry");
    let revision = capture["revision_id"].as_str().unwrap();
    let original_hash = capture["record"]["revisions"][0]["text_sha256"]
        .as_str()
        .unwrap();

    let queued: Value = serde_json::from_slice(
        &babata(&temp)
            .env("BABATA_BAILIAN_CLI", &fake_bl)
            .args([
                "--json",
                "process",
                "enqueue",
                "--pipeline",
                "bailian_summary",
                "--revision",
                revision,
            ])
            .assert()
            .success()
            .get_output()
            .stdout,
    )
    .unwrap();
    let first_job = queued["job"]["id"].as_str().unwrap();
    let failed: Value = serde_json::from_slice(
        &babata(&temp)
            .env("BABATA_BAILIAN_CLI", &fake_bl)
            .env("BABATA_TEST_PROVIDER_FAIL", "1")
            .args(["--json", "process", "run-once"])
            .assert()
            .success()
            .get_output()
            .stdout,
    )
    .unwrap();
    assert_eq!(failed["status"], "failed");
    assert_eq!(failed["job"]["id"], first_job);
    assert_eq!(failed["job"]["attempt"], 1);
    assert_eq!(failed["job"]["error_code"], "provider_failed");
    assert!(failed["job"]["result_run_id"].as_str().is_some());

    let retry: Value = serde_json::from_slice(
        &babata(&temp)
            .env("BABATA_BAILIAN_CLI", &fake_bl)
            .args(["--json", "process", "retry", first_job])
            .assert()
            .success()
            .get_output()
            .stdout,
    )
    .unwrap();
    assert_eq!(retry["job"]["attempt"], 2);
    assert_eq!(retry["job"]["retry_of_job_id"], first_job);

    let succeeded: Value = serde_json::from_slice(
        &babata(&temp)
            .env("BABATA_BAILIAN_CLI", &fake_bl)
            .args(["--json", "process", "run-once"])
            .assert()
            .success()
            .get_output()
            .stdout,
    )
    .unwrap();
    assert_eq!(succeeded["status"], "succeeded");
    assert_eq!(succeeded["job"]["attempt"], 2);
    assert_eq!(succeeded["job"]["retry_of_job_id"], first_job);

    let runs: Value = serde_json::from_slice(
        &babata(&temp)
            .args(["--json", "process", "list-runs", "--revision", revision])
            .assert()
            .success()
            .get_output()
            .stdout,
    )
    .unwrap();
    let runs = runs.as_array().unwrap();
    assert_eq!(runs.len(), 2);
    assert_eq!(runs[0]["state"], "failed");
    assert_eq!(runs[1]["state"], "succeeded");
    assert_eq!(runs[1]["retry_of_run_id"], runs[0]["id"]);
    assert_eq!(runs[1]["params"]["queue_job_id"], succeeded["job"]["id"]);
    assert_eq!(runs[1]["params"]["provider_task_id"], "fake-request");
    assert!(runs.iter().all(|run| run["input_sha256"] == original_hash));
}

#[test]
fn process_retry_rejects_a_succeeded_parent() {
    let temp = tempdir().unwrap();
    let capture = capture_text(&temp, "source text for cleaning");
    let revision = capture["revision_id"].as_str().unwrap();
    let sha = capture["record"]["revisions"][0]["text_sha256"]
        .as_str()
        .unwrap()
        .to_string();

    let first = babata(&temp)
        .args([
            "--json",
            "process",
            "register",
            "--pipeline",
            "agent_import",
            "--revision",
            revision,
            "--kind",
            "summary",
            "--provider",
            "bailian_cli",
            "--input-sha256",
            &sha,
            "--text",
            "first summary",
            "--model",
            "qwen-plus",
            "--tool-version",
            "1.10.0",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let first: Value = serde_json::from_slice(&first).unwrap();
    let run_id = first["run_id"].as_str().unwrap().to_string();

    let second = babata(&temp)
        .args([
            "--json",
            "process",
            "register",
            "--pipeline",
            "agent_import",
            "--revision",
            revision,
            "--kind",
            "summary",
            "--provider",
            "bailian_cli",
            "--input-sha256",
            &sha,
            "--text",
            "retry summary",
            "--retry-of",
            &run_id,
            "--model",
            "qwen-plus",
            "--tool-version",
            "1.10.0",
        ])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let second: Value = serde_json::from_slice(&second).unwrap();
    assert!(
        second["message"]
            .as_str()
            .unwrap()
            .contains("is not failed")
    );

    let runs = babata(&temp)
        .args(["--json", "process", "list-runs", "--revision", revision])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let runs: Value = serde_json::from_slice(&runs).unwrap();
    assert_eq!(runs.as_array().unwrap().len(), 1);
    assert_eq!(runs[0]["input_item_id"], capture["item_id"]);
}

#[test]
fn process_register_rejects_wrong_input_hash() {
    let temp = tempdir().unwrap();
    let capture = capture_text(&temp, "hash binding check");
    let revision = capture["revision_id"].as_str().unwrap();
    let wrong = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    let output = babata(&temp)
        .args([
            "--json",
            "process",
            "register",
            "--pipeline",
            "agent_import",
            "--revision",
            revision,
            "--kind",
            "summary",
            "--provider",
            "bailian_cli",
            "--input-sha256",
            wrong,
            "--text",
            "orphan summary",
            "--model",
            "qwen-plus",
            "--tool-version",
            "1.10.0",
        ])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let value: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(value["code"], "integrity_failed");
}

#[test]
fn process_register_binds_media_asset_and_imports_output_file() {
    let temp = tempdir().unwrap();
    let media = temp.path().join("frame.png");
    std::fs::write(&media, b"fake-png-bytes").unwrap();
    let capture = capture_file(&temp, &media);
    let revision = capture["revision_id"].as_str().unwrap();
    let asset_id = capture["asset_ids"][0].as_str().unwrap();
    let asset_sha = capture["record"]["assets"][0]["sha256"].as_str().unwrap();

    let staging_dir = temp.path().join("generated/demo/results");
    std::fs::create_dir_all(&staging_dir).unwrap();
    let ocr_file = staging_dir.join("ocr.md");
    std::fs::write(&ocr_file, "recognized blackboard text").unwrap();

    let registered = babata(&temp)
        .args([
            "--json",
            "process",
            "register",
            "--pipeline",
            "agent_import",
            "--revision",
            revision,
            "--kind",
            "ocr_text",
            "--provider",
            "bailian_cli",
            "--input-sha256",
            asset_sha,
            "--input-asset-id",
            asset_id,
            "--output-file",
            &ocr_file.to_string_lossy(),
            "--model",
            "qwen-vl-plus",
            "--tool-version",
            "1.10.0",
            "--usage-json",
            "{\"input_tokens\":12}",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let registered: Value = serde_json::from_slice(&registered).unwrap();
    let run_id = registered["run_id"].as_str().unwrap();

    let shown = babata(&temp)
        .args(["--json", "process", "show-run", "--run", run_id])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let shown: Value = serde_json::from_slice(&shown).unwrap();
    let derivative = &shown["derivatives"][0];
    assert_eq!(derivative["input_asset_id"], asset_id);
    let logical_path = derivative["logical_path"].as_str().unwrap();
    assert!(logical_path.starts_with("02_derived/files/sha256/"));
    assert_eq!(
        derivative["output_sha256"].as_str().unwrap(),
        sha256_hex(b"recognized blackboard text")
    );
    assert_eq!(shown["run"]["pipeline_id"], "agent_import");
    assert_eq!(shown["run"]["target_kind"], "ocr_text");
    assert_eq!(shown["run"]["input_asset_id"], asset_id);
    assert_eq!(shown["run"]["usage"]["input_tokens"], 12);
    // managed C1 file really landed under the data root
    assert!(temp.path().join(logical_path).exists());
}

#[test]
fn process_failed_run_then_successful_retry_keeps_both() {
    let temp = tempdir().unwrap();
    let media = temp.path().join("lecture.mp4");
    std::fs::write(&media, b"fake-video-bytes").unwrap();
    let capture = capture_file(&temp, &media);
    let revision = capture["revision_id"].as_str().unwrap();
    let asset_id = capture["asset_ids"][0].as_str().unwrap();
    let sha = capture["record"]["assets"][0]["sha256"].as_str().unwrap();

    let failed = babata(&temp)
        .args([
            "--json",
            "process",
            "register-failure",
            "--pipeline",
            "bailian_transcript",
            "--revision",
            revision,
            "--kind",
            "transcript",
            "--provider",
            "bailian_cli",
            "--input-sha256",
            sha,
            "--input-asset-id",
            asset_id,
            "--model",
            "fun-asr",
            "--tool-version",
            "1.10.0",
            "--error-code",
            "provider_timeout",
            "--error-message",
            "ASR provider timed out",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let failed: Value = serde_json::from_slice(&failed).unwrap();
    assert_eq!(failed["state"], "failed");
    assert!(failed["derivative_id"].is_null());
    let failed_run = failed["run_id"].as_str().unwrap().to_string();

    let retried = babata(&temp)
        .args([
            "--json",
            "process",
            "register",
            "--pipeline",
            "bailian_transcript",
            "--revision",
            revision,
            "--kind",
            "transcript",
            "--provider",
            "bailian_cli",
            "--input-sha256",
            sha,
            "--input-asset-id",
            asset_id,
            "--text",
            "clean transcript",
            "--retry-of",
            &failed_run,
            "--model",
            "fun-asr",
            "--tool-version",
            "1.10.0",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let retried: Value = serde_json::from_slice(&retried).unwrap();
    assert_eq!(retried["state"], "succeeded");

    let runs = babata(&temp)
        .args(["--json", "process", "list-runs", "--revision", revision])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let runs: Value = serde_json::from_slice(&runs).unwrap();
    assert_eq!(runs.as_array().unwrap().len(), 2);
    assert_eq!(runs[0]["state"], "failed");
    assert_eq!(runs[0]["error_code"], "provider_timeout");
    assert_eq!(runs[0]["target_kind"], "transcript");
    assert_eq!(runs[0]["input_asset_id"], asset_id);
    assert_eq!(runs[1]["state"], "succeeded");
    assert_eq!(runs[1]["attempt"], 2);
    assert_eq!(runs[1]["retry_of_run_id"], failed_run);
}

#[test]
fn process_retry_rejects_mismatched_parent_input() {
    let temp = tempdir().unwrap();
    let first_capture = capture_text(&temp, "first source");
    let first_revision = first_capture["revision_id"].as_str().unwrap().to_string();
    let first_sha = first_capture["record"]["revisions"][0]["text_sha256"]
        .as_str()
        .unwrap()
        .to_string();
    let second_capture = capture_text(&temp, "second source");
    let second_revision = second_capture["revision_id"].as_str().unwrap().to_string();

    let registered = babata(&temp)
        .args([
            "--json",
            "process",
            "register-failure",
            "--pipeline",
            "agent_import",
            "--revision",
            &first_revision,
            "--kind",
            "summary",
            "--provider",
            "bailian_cli",
            "--input-sha256",
            &first_sha,
            "--model",
            "qwen-plus",
            "--tool-version",
            "1.10.0",
            "--error-code",
            "provider_timeout",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let registered: Value = serde_json::from_slice(&registered).unwrap();
    let run_id = registered["run_id"].as_str().unwrap().to_string();

    // retrying under a different revision must fail
    babata(&temp)
        .args([
            "--json",
            "process",
            "register",
            "--pipeline",
            "agent_import",
            "--revision",
            &second_revision,
            "--kind",
            "summary",
            "--provider",
            "bailian_cli",
            "--input-sha256",
            &first_sha,
            "--text",
            "cross retry",
            "--retry-of",
            &run_id,
            "--model",
            "qwen-plus",
            "--tool-version",
            "1.10.0",
        ])
        .assert()
        .failure();

    let runs = babata(&temp)
        .args([
            "--json",
            "process",
            "list-runs",
            "--revision",
            &first_revision,
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let runs: Value = serde_json::from_slice(&runs).unwrap();
    assert_eq!(runs.as_array().unwrap().len(), 1);
}

#[test]
fn process_media_derivative_rejects_text_revision_without_asset() {
    let temp = tempdir().unwrap();
    let capture = capture_text(&temp, "not a video asset");
    let revision = capture["revision_id"].as_str().unwrap();
    let sha = capture["record"]["revisions"][0]["text_sha256"]
        .as_str()
        .unwrap();

    let output = babata(&temp)
        .args([
            "--json",
            "process",
            "register",
            "--pipeline",
            "bailian_transcript",
            "--revision",
            revision,
            "--kind",
            "transcript",
            "--provider",
            "bailian_cli",
            "--input-sha256",
            sha,
            "--text",
            "impossible transcript",
            "--model",
            "fun-asr",
            "--tool-version",
            "1.10.0",
        ])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let output: Value = serde_json::from_slice(&output).unwrap();
    assert!(
        output["message"]
            .as_str()
            .unwrap()
            .contains("require --input-asset-id")
    );
}

#[test]
fn process_c0_validation_failure_does_not_import_output_file() {
    let temp = tempdir().unwrap();
    let capture = capture_text(&temp, "valid source");
    let revision = capture["revision_id"].as_str().unwrap();
    let output_file = temp.path().join("generated/summary.md");
    std::fs::create_dir_all(output_file.parent().unwrap()).unwrap();
    std::fs::write(&output_file, "summary bytes").unwrap();

    babata(&temp)
        .args([
            "--json",
            "process",
            "register",
            "--pipeline",
            "agent_import",
            "--revision",
            revision,
            "--kind",
            "summary",
            "--provider",
            "bailian_cli",
            "--input-sha256",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "--output-file",
            &output_file.to_string_lossy(),
            "--model",
            "qwen-plus",
            "--tool-version",
            "1.10.0",
        ])
        .assert()
        .failure();

    assert_eq!(file_count(&temp.path().join("02_derived/files")), 0);
}

#[test]
fn process_conflicting_output_representations_are_rejected_and_cleaned() {
    let temp = tempdir().unwrap();
    let capture = capture_text(&temp, "valid source");
    let revision = capture["revision_id"].as_str().unwrap();
    let sha = capture["record"]["revisions"][0]["text_sha256"]
        .as_str()
        .unwrap();
    let output_file = temp.path().join("generated/summary.md");
    std::fs::create_dir_all(output_file.parent().unwrap()).unwrap();
    std::fs::write(&output_file, "file A").unwrap();

    let output = babata(&temp)
        .args([
            "--json",
            "process",
            "register",
            "--pipeline",
            "agent_import",
            "--revision",
            revision,
            "--kind",
            "summary",
            "--provider",
            "bailian_cli",
            "--input-sha256",
            sha,
            "--text",
            "text B",
            "--output-file",
            &output_file.to_string_lossy(),
            "--model",
            "qwen-plus",
            "--tool-version",
            "1.10.0",
        ])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let output: Value = serde_json::from_slice(&output).unwrap();
    assert!(
        output["message"]
            .as_str()
            .unwrap()
            .contains("output representations disagree")
    );
    assert_eq!(file_count(&temp.path().join("02_derived/files")), 0);
    assert_eq!(file_count(&temp.path().join("04_runtime/asset-journal")), 0);
}

#[test]
fn process_rejects_generated_path_as_formal_c1_storage() {
    let temp = tempdir().unwrap();
    let capture = capture_text(&temp, "valid source");
    let revision = capture["revision_id"].as_str().unwrap();
    let sha = capture["record"]["revisions"][0]["text_sha256"]
        .as_str()
        .unwrap();

    let output = babata(&temp)
        .args([
            "--json",
            "process",
            "register",
            "--pipeline",
            "agent_import",
            "--revision",
            revision,
            "--kind",
            "summary",
            "--provider",
            "bailian_cli",
            "--input-sha256",
            sha,
            "--logical-path",
            "generated/staging-summary.md",
            "--model",
            "qwen-plus",
            "--tool-version",
            "1.10.0",
        ])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let output: Value = serde_json::from_slice(&output).unwrap();
    assert!(
        output["message"]
            .as_str()
            .unwrap()
            .contains("must be under 02_derived/files/sha256")
    );
}

#[test]
fn process_retry_rejects_a_different_target_kind() {
    let temp = tempdir().unwrap();
    let capture = capture_text(&temp, "retry source");
    let revision = capture["revision_id"].as_str().unwrap();
    let sha = capture["record"]["revisions"][0]["text_sha256"]
        .as_str()
        .unwrap();
    let failed = babata(&temp)
        .args([
            "--json",
            "process",
            "register-failure",
            "--pipeline",
            "agent_import",
            "--revision",
            revision,
            "--kind",
            "summary",
            "--provider",
            "bailian_cli",
            "--input-sha256",
            sha,
            "--model",
            "qwen-plus",
            "--tool-version",
            "1.10.0",
            "--error-code",
            "provider_timeout",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let failed: Value = serde_json::from_slice(&failed).unwrap();

    let output = babata(&temp)
        .args([
            "--json",
            "process",
            "register",
            "--pipeline",
            "agent_import",
            "--revision",
            revision,
            "--kind",
            "tags",
            "--provider",
            "bailian_cli",
            "--input-sha256",
            sha,
            "--text",
            "tag-a",
            "--model",
            "qwen-plus",
            "--tool-version",
            "1.10.0",
            "--retry-of",
            failed["run_id"].as_str().unwrap(),
        ])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let output: Value = serde_json::from_slice(&output).unwrap();
    assert!(
        output["message"]
            .as_str()
            .unwrap()
            .contains("target kind does not match")
    );
}

#[test]
fn process_commit_failure_preserves_recoverable_c1_file_evidence() {
    let temp = tempdir().unwrap();
    let capture = capture_text(&temp, "valid source");
    let revision = capture["revision_id"].as_str().unwrap();
    let sha = capture["record"]["revisions"][0]["text_sha256"]
        .as_str()
        .unwrap();
    let output_file = temp.path().join("generated/summary.md");
    std::fs::create_dir_all(output_file.parent().unwrap()).unwrap();
    std::fs::write(&output_file, "recoverable summary").unwrap();

    let output = babata(&temp)
        .env("BABATA_TEST_DERIVED_FAULT", "commit")
        .args([
            "--json",
            "process",
            "register",
            "--pipeline",
            "agent_import",
            "--revision",
            revision,
            "--kind",
            "summary",
            "--provider",
            "bailian_cli",
            "--input-sha256",
            sha,
            "--output-file",
            &output_file.to_string_lossy(),
            "--model",
            "qwen-plus",
            "--tool-version",
            "1.10.0",
        ])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let output: Value = serde_json::from_slice(&output).unwrap();
    assert!(
        output["operation_id"]
            .as_str()
            .unwrap()
            .starts_with("c1_run_")
    );
    assert_eq!(file_count(&temp.path().join("02_derived/files")), 1);
    assert_eq!(file_count(&temp.path().join("04_runtime/asset-journal")), 1);
    assert_eq!(
        file_count(&temp.path().join("01_raw/quarantine/orphans")),
        1
    );

    let runs = babata(&temp)
        .args(["--json", "process", "list-runs", "--revision", revision])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let runs: Value = serde_json::from_slice(&runs).unwrap();
    assert!(runs.as_array().unwrap().is_empty());
}

#[test]
fn process_rejects_empty_text_and_invalid_json_before_creating_runs() {
    let temp = tempdir().unwrap();
    let capture = capture_text(&temp, "validation source");
    let revision = capture["revision_id"].as_str().unwrap();
    let sha = capture["record"]["revisions"][0]["text_sha256"]
        .as_str()
        .unwrap();
    let invalid_json = temp.path().join("invalid.json");
    std::fs::write(&invalid_json, "{not-json}").unwrap();

    for output_args in [
        vec!["--text", "   "],
        vec!["--json-file", invalid_json.to_str().unwrap()],
    ] {
        let mut args = vec![
            "--json",
            "process",
            "register",
            "--pipeline",
            "agent_import",
            "--revision",
            revision,
            "--kind",
            "summary",
            "--provider",
            "bailian_cli",
            "--input-sha256",
            sha,
            "--model",
            "qwen-plus",
            "--tool-version",
            "1.10.0",
        ];
        args.extend(output_args);
        babata(&temp).args(args).assert().failure();
    }

    let runs = babata(&temp)
        .args(["--json", "process", "list-runs", "--revision", revision])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let runs: Value = serde_json::from_slice(&runs).unwrap();
    assert!(runs.as_array().unwrap().is_empty());
}

#[test]
fn process_rejects_blank_provider_and_incompatible_pipeline_kind() {
    let temp = tempdir().unwrap();
    let capture = capture_text(&temp, "validation source");
    let revision = capture["revision_id"].as_str().unwrap();
    let sha = capture["record"]["revisions"][0]["text_sha256"]
        .as_str()
        .unwrap();

    babata(&temp)
        .args([
            "--json",
            "process",
            "register",
            "--pipeline",
            "agent_import",
            "--revision",
            revision,
            "--kind",
            "summary",
            "--provider",
            " ",
            "--input-sha256",
            sha,
            "--text",
            "summary",
            "--model",
            "qwen-plus",
            "--tool-version",
            "1.10.0",
        ])
        .assert()
        .failure();

    let output = babata(&temp)
        .args([
            "--json",
            "process",
            "register",
            "--pipeline",
            "bailian_summary",
            "--revision",
            revision,
            "--kind",
            "tags",
            "--provider",
            "bailian_cli",
            "--input-sha256",
            sha,
            "--text",
            "tag-a",
            "--model",
            "qwen-plus",
            "--tool-version",
            "1.10.0",
        ])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let output: Value = serde_json::from_slice(&output).unwrap();
    assert!(
        output["message"]
            .as_str()
            .unwrap()
            .contains("cannot produce tags")
    );
}

#[test]
fn process_delete_result_then_rebuild_preserves_c0_and_history() {
    let temp = tempdir().unwrap();
    let source = temp.path().join("source.pdf");
    std::fs::write(&source, b"real-pdf-source-bytes").unwrap();
    let capture = capture_file(&temp, &source);
    let revision = capture["revision_id"].as_str().unwrap();
    let item = capture["item_id"].as_str().unwrap();
    let asset = &capture["record"]["assets"][0];
    let asset_id = capture["asset_ids"][0].as_str().unwrap();
    let input_sha = asset["sha256"].as_str().unwrap();
    let raw_path = temp.path().join(asset["logical_path"].as_str().unwrap());
    let c0_before = std::fs::read(&raw_path).unwrap();
    let output = temp.path().join("generated/summary.md");
    std::fs::create_dir_all(output.parent().unwrap()).unwrap();
    std::fs::write(&output, "rebuildable summary").unwrap();

    let register = || {
        let bytes = babata(&temp)
            .args([
                "--json",
                "process",
                "register",
                "--pipeline",
                "agent_import",
                "--revision",
                revision,
                "--item",
                item,
                "--kind",
                "summary",
                "--provider",
                "local_extract",
                "--model",
                "fixture",
                "--tool-version",
                "1.0.0",
                "--input-sha256",
                input_sha,
                "--input-asset-id",
                asset_id,
                "--text-file",
                output.to_str().unwrap(),
                "--output-file",
                output.to_str().unwrap(),
            ])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        serde_json::from_slice::<Value>(&bytes).unwrap()
    };

    let first = register();
    let first_run = first["run_id"].as_str().unwrap();
    let deleted = babata(&temp)
        .args([
            "--json",
            "process",
            "delete-result",
            "--run",
            first_run,
            "--reason",
            "TC-03A deletion and rebuild",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let deleted: Value = serde_json::from_slice(&deleted).unwrap();
    assert!(deleted["run"]["invalidated_at"].as_str().is_some());
    assert_eq!(
        deleted["run"]["invalidation_reason"],
        "TC-03A deletion and rebuild"
    );
    assert_eq!(deleted["derivatives"].as_array().unwrap().len(), 1);
    assert_eq!(std::fs::read(&raw_path).unwrap(), c0_before);
    assert_eq!(sha256_hex(&c0_before), input_sha);

    let second = register();
    assert_ne!(second["run_id"], first["run_id"]);
    let runs = babata(&temp)
        .args(["--json", "process", "list-runs", "--revision", revision])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let runs: Value = serde_json::from_slice(&runs).unwrap();
    assert_eq!(runs.as_array().unwrap().len(), 2);
    assert!(runs[0]["invalidated_at"].as_str().is_some());
    assert!(runs[1]["invalidated_at"].is_null());
    assert_eq!(std::fs::read(&raw_path).unwrap(), c0_before);
}

#[test]
fn process_delete_result_rejects_failed_run() {
    let temp = tempdir().unwrap();
    let capture = capture_text(&temp, "failed source");
    let revision = capture["revision_id"].as_str().unwrap();
    let input_sha = capture["record"]["revisions"][0]["text_sha256"]
        .as_str()
        .unwrap();
    let failed = babata(&temp)
        .args([
            "--json",
            "process",
            "register-failure",
            "--pipeline",
            "agent_import",
            "--revision",
            revision,
            "--kind",
            "summary",
            "--provider",
            "fixture",
            "--model",
            "fixture",
            "--tool-version",
            "1.0.0",
            "--input-sha256",
            input_sha,
            "--error-code",
            "provider_failure",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let failed: Value = serde_json::from_slice(&failed).unwrap();

    let output = babata(&temp)
        .args([
            "--json",
            "process",
            "delete-result",
            "--run",
            failed["run_id"].as_str().unwrap(),
            "--reason",
            "not a successful result",
        ])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let output: Value = serde_json::from_slice(&output).unwrap();
    assert!(
        output["message"]
            .as_str()
            .unwrap()
            .contains("only succeeded C1 results can be deleted")
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn knowledge_review_prepares_c0_c1_without_manual_semantic_writes() {
    let temp = tempdir().unwrap();
    let capture = capture_text(&temp, "source evidence for P6.1");
    let item = capture["item_id"].as_str().unwrap();
    let revision = capture["revision_id"].as_str().unwrap();
    let input_sha = capture["record"]["revisions"][0]["text_sha256"]
        .as_str()
        .unwrap();
    let machine_summary = temp.path().join("machine-summary.txt");
    std::fs::write(&machine_summary, "machine summary").unwrap();
    babata(&temp)
        .args([
            "--json",
            "process",
            "register",
            "--pipeline",
            "agent_import",
            "--revision",
            revision,
            "--kind",
            "summary",
            "--provider",
            "fixture",
            "--model",
            "fixture-model",
            "--tool-version",
            "1.0.0",
            "--input-sha256",
            input_sha,
            "--text-file",
            machine_summary.to_str().unwrap(),
            "--output-file",
            machine_summary.to_str().unwrap(),
        ])
        .assert()
        .success();

    let review = babata(&temp)
        .args([
            "--json",
            "knowledge",
            "review",
            "--item",
            item,
            "--revision",
            revision,
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let review: Value = serde_json::from_slice(&review).unwrap();
    assert_eq!(review["target_revision_id"], revision);
    assert_eq!(review["process_runs"].as_array().unwrap().len(), 1);
    assert_eq!(
        review["process_runs"][0]["derivatives"][0]["content_text"],
        "machine summary"
    );
    assert!(review.get("knowledge_records").is_none());

    let other = capture_text(&temp, "unrelated item");
    babata(&temp)
        .args([
            "--json",
            "knowledge",
            "review",
            "--item",
            other["item_id"].as_str().unwrap(),
            "--revision",
            revision,
        ])
        .assert()
        .failure();

    let managed_output = review["process_runs"][0]["derivatives"][0]["logical_path"]
        .as_str()
        .unwrap();
    std::fs::write(temp.path().join(managed_output), "tampered summary").unwrap();
    let invalid = babata(&temp)
        .args([
            "--json",
            "knowledge",
            "review",
            "--item",
            item,
            "--revision",
            revision,
        ])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let invalid: Value = serde_json::from_slice(&invalid).unwrap();
    assert!(
        invalid["message"]
            .as_str()
            .unwrap()
            .contains("output no longer matches its hash")
    );

    for retired in ["create", "revise"] {
        let output = babata(&temp)
            .args(["knowledge", retired])
            .assert()
            .failure()
            .get_output()
            .stderr
            .clone();
        assert!(
            String::from_utf8_lossy(&output).contains("unrecognized subcommand"),
            "{retired} must not remain a Knowledge subcommand"
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn p6_semantic_candidate_enters_three_realm_core_without_source_v2() {
    let temp = tempdir().unwrap();
    let capture = capture_text(
        &temp,
        "crawler engineering source with a concrete repository case",
    );
    let item = capture["item_id"].as_str().unwrap();
    let revision = capture["revision_id"].as_str().unwrap();
    let input_sha = capture["record"]["revisions"][0]["text_sha256"]
        .as_str()
        .unwrap();
    let summary_text = "machine summary evidence";
    let summary = babata(&temp)
        .args([
            "--json",
            "process",
            "register",
            "--pipeline",
            "agent_import",
            "--revision",
            revision,
            "--kind",
            "summary",
            "--provider",
            "fixture",
            "--model",
            "fixture-model",
            "--tool-version",
            "1.0.0",
            "--input-sha256",
            input_sha,
            "--text",
            summary_text,
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let summary: Value = serde_json::from_slice(&summary).unwrap();
    let summary_derivative = summary["derivative_id"].as_str().unwrap();
    let package = serde_json::json!({
        "schema_version": "p6-semantic-candidate/v1",
        "source_item_id": item,
        "source_revision_id": revision,
        "evidence_derivatives": [{
            "derivative_id": summary_derivative,
            "output_sha256": sha256_hex(summary_text.as_bytes())
        }],
        "provider": "fixture",
        "model": "fixture-semantic-model",
        "model_version": "1.0.0",
        "prompt_version": "p6-semantic-v1",
        "generated_at": "2026-07-21T00:00:00Z",
        "map_nodes": [
            {
                "local_ref": "node:software-engineering",
                "level": "discipline",
                "name": "软件工程",
                "parent_refs": ["foundation:matter", "foundation:consciousness"]
            },
            {
                "local_ref": "node:web-collection",
                "level": "branch",
                "name": "网络数据采集",
                "parent_refs": ["node:software-engineering"]
            }
        ],
        "entries": [
            {
                "local_ref": "entry:knowledge",
                "title": "爬虫工程的降维策略",
                "payload": {
                    "kind": "knowledge",
                    "statement": "优先复用成熟工具链，再补最窄缺口。",
                    "details": "仓库案例共同表明，发现、认证、下载和结构化应分别选择成熟能力。"
                },
                "map_node_refs": ["foundation:matter", "foundation:consciousness", "node:web-collection"],
                "tags": ["爬虫", "工具复用"],
                "dense_expressions": [{
                    "kind": "outline",
                    "content": "1. 发现候选\n2. 复用工具\n3. 仅补缺口\n4. 保留证据"
                }],
                "relevance": {"interest": 80, "strategy": 60, "consensus": 40, "rationale": "当前采集工作直接相关"}
            },
            {
                "local_ref": "entry:case",
                "title": "二十个爬虫仓库的工具组合案例",
                "payload": {
                    "kind": "case",
                    "scenario": "为多平台资料收集选择实现路线",
                    "process": "比较二十个仓库的发现、认证和下载能力",
                    "result": "形成优先复用成熟工具的路线",
                    "analysis": "单站点重复开发的维护成本更高"
                },
                "map_node_refs": ["node:web-collection"],
                "tags": ["爬虫", "案例"],
                "dense_expressions": [],
                "relevance": {"interest": 70, "strategy": 65, "consensus": 35, "rationale": "提供实践证据"}
            }
        ],
        "relations": [
            {"from_ref": "entry:case", "kind": "validates", "to_ref": "entry:knowledge", "evidence": "仓库比较支持该工程策略"},
            {"from_ref": "entry:knowledge", "kind": "applied_by", "to_ref": "entry:case", "evidence": "案例应用了该策略"}
        ],
        "limitations": ["fixture only"]
    });
    let package_path = temp.path().join("semantic-package.json");
    std::fs::write(&package_path, package.to_string()).unwrap();
    let registered = babata(&temp)
        .args([
            "--json",
            "process",
            "register",
            "--pipeline",
            "agent_import",
            "--revision",
            revision,
            "--kind",
            "structured_result",
            "--provider",
            "fixture",
            "--model",
            "fixture-semantic-model",
            "--tool-version",
            "1.0.0",
            "--input-sha256",
            input_sha,
            "--json-file",
            package_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let registered: Value = serde_json::from_slice(&registered).unwrap();
    let semantic_derivative = registered["derivative_id"].as_str().unwrap();
    let ingested = babata(&temp)
        .args([
            "--json",
            "knowledge",
            "ingest",
            "--derivative",
            semantic_derivative,
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let ingested: Value = serde_json::from_slice(&ingested).unwrap();
    assert_eq!(ingested["review_state"], "unreviewed");
    assert_eq!(ingested["semantic_ids"].as_array().unwrap().len(), 2);
    assert!(ingested["map_node_ids"].as_array().unwrap().len() >= 4);
    let suggestion = ingested["suggestion_id"].as_str().unwrap();

    babata(&temp)
        .args([
            "--json",
            "knowledge",
            "ingest",
            "--derivative",
            semantic_derivative,
        ])
        .assert()
        .failure();

    let show = || {
        let bytes = babata(&temp)
            .args(["--json", "knowledge", "show", "--suggestion", suggestion])
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        serde_json::from_slice::<Value>(&bytes).unwrap()
    };
    let snapshot = show();
    assert_eq!(snapshot["suggestion"]["review_state"], "unreviewed");
    assert!(
        snapshot["suggestion"]["downstream_eligibility"]["eligible_uses"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value == "search")
    );
    assert_eq!(
        snapshot["suggestion"]["downstream_eligibility"]["human_judgment"],
        false
    );
    assert_eq!(snapshot["entries"].as_array().unwrap().len(), 2);
    assert_eq!(snapshot["relations"].as_array().unwrap().len(), 2);
    assert!(snapshot["entries"].as_array().unwrap().iter().any(|entry| {
        entry["kind"] == "knowledge"
            && entry["realm"] == "knowledge_and_cases"
            && entry["dense_expressions"].as_array().unwrap().len() == 1
            && entry["scores"][0]["weighted_score"] == 6300
    }));
    let dense_semantic = snapshot["entries"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| !entry["dense_expressions"].as_array().unwrap().is_empty())
        .unwrap()["semantic_id"]
        .as_str()
        .unwrap();
    let built_preview = babata(&temp)
        .args([
            "--json",
            "knowledge",
            "build-dense-preview",
            "--semantic",
            dense_semantic,
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let built_preview: Value = serde_json::from_slice(&built_preview).unwrap();
    assert_eq!(built_preview["status"], "built");
    assert!(built_preview["source_sha256"].is_string());
    assert!(built_preview["output_sha256"].is_string());
    let preview_directory = temp.path().join("03_views/p6_dense").join(dense_semantic);
    let preview_path = preview_directory.join("preview.md");
    assert!(preview_path.exists());
    babata(&temp)
        .args([
            "--json",
            "knowledge",
            "verify-dense-preview",
            "--semantic",
            dense_semantic,
        ])
        .assert()
        .success();
    std::fs::write(&preview_path, "tampered optional view").unwrap();
    babata(&temp)
        .args([
            "--json",
            "knowledge",
            "verify-dense-preview",
            "--semantic",
            dense_semantic,
        ])
        .assert()
        .failure();
    let rebuilt_preview = babata(&temp)
        .args([
            "--json",
            "knowledge",
            "build-dense-preview",
            "--semantic",
            dense_semantic,
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let rebuilt_preview: Value = serde_json::from_slice(&rebuilt_preview).unwrap();
    assert_eq!(rebuilt_preview["status"], "rebuilt");
    babata(&temp)
        .args([
            "--json",
            "knowledge",
            "delete-dense-preview",
            "--semantic",
            dense_semantic,
        ])
        .assert()
        .success();
    assert!(!preview_directory.exists());
    let entry_after_delete = babata(&temp)
        .args([
            "--json",
            "knowledge",
            "show-entry",
            "--semantic",
            dense_semantic,
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let entry_after_delete: Value = serde_json::from_slice(&entry_after_delete).unwrap();
    assert!(
        !entry_after_delete["dense_expressions"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    babata(&temp)
        .args([
            "--json",
            "knowledge",
            "build-dense-preview",
            "--semantic",
            dense_semantic,
        ])
        .assert()
        .success();
    assert!(preview_path.exists());

    let create_map_node = |level: &str, name: &str, parents: &[&str], tags: &[&str]| {
        let mut args = vec![
            "--json",
            "knowledge",
            "create-map-node",
            "--level",
            level,
            "--name",
            name,
            "--rationale",
            "temporary P6 map evolution proof",
        ];
        for parent in parents {
            args.extend(["--parent", parent]);
        }
        for tag in tags {
            args.extend(["--tag", tag]);
        }
        let bytes = babata(&temp)
            .args(args)
            .assert()
            .success()
            .get_output()
            .stdout
            .clone();
        serde_json::from_slice::<Value>(&bytes).unwrap()
    };
    let discipline_a = create_map_node(
        "discipline",
        "系统研究",
        &["mapnode_p6_matter", "mapnode_p6_consciousness"],
        &["系统"],
    );
    assert_eq!(discipline_a["parent_node_ids"].as_array().unwrap().len(), 2);
    assert_eq!(discipline_a["node_events"][0]["kind"], "created");
    assert_eq!(discipline_a["edge_events"].as_array().unwrap().len(), 2);
    assert_eq!(discipline_a["tag_events"][0]["kind"], "assigned");
    let systems_discipline_id = discipline_a["map_node_id"].as_str().unwrap();
    let discipline_b = create_map_node("discipline", "知识工程", &["mapnode_p6_matter"], &[]);
    let knowledge_discipline_id = discipline_b["map_node_id"].as_str().unwrap();
    let branch = create_map_node("branch", "语义流水线", &[systems_discipline_id], &["P6"]);
    let branch_id = branch["map_node_id"].as_str().unwrap();
    let target_branch = create_map_node("branch", "知识流水线", &[knowledge_discipline_id], &[]);
    let target_branch_id = target_branch["map_node_id"].as_str().unwrap();

    let node_score = babata(&temp)
        .args([
            "--json",
            "knowledge",
            "score",
            "--map-node",
            branch_id,
            "--interest",
            "75",
            "--strategy",
            "85",
            "--consensus",
            "45",
            "--rationale",
            "map node relevance proof",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let node_score: Value = serde_json::from_slice(&node_score).unwrap();
    assert_eq!(node_score["target_kind"], "map_node");
    assert_eq!(node_score["author"], "user");
    assert_eq!(node_score["provenance_kind"], "first_party");
    assert!(node_score["created_at"].is_string());

    babata(&temp)
        .args([
            "--json",
            "knowledge",
            "rename-map-node",
            "--map-node",
            branch_id,
            "--name",
            "语义沉淀流水线",
            "--rationale",
            "name now reflects the actual scope",
        ])
        .assert()
        .success();
    babata(&temp)
        .args([
            "--json",
            "knowledge",
            "change-map-parent",
            "--parent",
            knowledge_discipline_id,
            "--child",
            branch_id,
            "--change",
            "assign",
            "--rationale",
            "branch now spans two disciplines",
        ])
        .assert()
        .success();
    babata(&temp)
        .args([
            "--json",
            "knowledge",
            "change-map-parent",
            "--parent",
            systems_discipline_id,
            "--child",
            branch_id,
            "--change",
            "unassign",
            "--rationale",
            "move the branch without losing history",
        ])
        .assert()
        .success();
    babata(&temp)
        .args([
            "--json",
            "knowledge",
            "change-map-assignment",
            "--semantic",
            ingested["semantic_ids"][0].as_str().unwrap(),
            "--map-node",
            branch_id,
            "--change",
            "assign",
            "--rationale",
            "explicit first-party map placement",
        ])
        .assert()
        .success();
    babata(&temp)
        .args([
            "--json",
            "knowledge",
            "tag-map-node",
            "--map-node",
            branch_id,
            "--tag",
            "P6",
            "--change",
            "unassign",
            "--rationale",
            "replace a temporary map tag",
        ])
        .assert()
        .success();
    babata(&temp)
        .args([
            "--json",
            "knowledge",
            "merge-map-node",
            "--map-node",
            branch_id,
            "--into",
            target_branch_id,
            "--rationale",
            "two branches now represent the same concept",
        ])
        .assert()
        .success();
    let merged = babata(&temp)
        .args([
            "--json",
            "knowledge",
            "show-map-node",
            "--map-node",
            branch_id,
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let merged: Value = serde_json::from_slice(&merged).unwrap();
    assert_eq!(merged["lifecycle"], "merged");
    assert!(
        merged["node_events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| {
                event["kind"] == "renamed"
                    && event["previous_name"] == "语义流水线"
                    && event["current_name"] == "语义沉淀流水线"
            })
    );
    assert!(
        merged["node_events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| {
                event["kind"] == "merged" && event["merged_into_map_node_id"] == target_branch_id
            })
    );
    assert!(
        merged["edge_events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| {
                event["kind"] == "unassigned" && event["parent_node_id"] == systems_discipline_id
            })
    );
    let merge_target = babata(&temp)
        .args([
            "--json",
            "knowledge",
            "show-map-node",
            "--map-node",
            target_branch_id,
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let merge_target: Value = serde_json::from_slice(&merge_target).unwrap();
    assert!(
        merge_target["semantic_ids"]
            .as_array()
            .unwrap()
            .iter()
            .any(|id| id.as_str() == ingested["semantic_ids"][0].as_str())
    );
    assert!(
        merge_target["assignment_events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| event["kind"] == "assigned")
    );

    babata(&temp)
        .args([
            "--json",
            "knowledge",
            "deactivate-map-node",
            "--map-node",
            knowledge_discipline_id,
            "--rationale",
            "active children must block deactivation",
        ])
        .assert()
        .failure();

    babata(&temp)
        .args([
            "--json",
            "knowledge",
            "deactivate-map-node",
            "--map-node",
            systems_discipline_id,
            "--rationale",
            "discipline is no longer current",
        ])
        .assert()
        .success();
    babata(&temp)
        .args([
            "--json",
            "knowledge",
            "rename-map-node",
            "--map-node",
            "mapnode_p6_time",
            "--name",
            "时间轴",
            "--rationale",
            "foundation mutation must fail",
        ])
        .assert()
        .failure();

    let profiles = babata(&temp)
        .args(["--json", "knowledge", "list-profiles"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let profiles: Value = serde_json::from_slice(&profiles).unwrap();
    assert_eq!(profiles[0]["interest_weight"], 40);
    babata(&temp)
        .args([
            "--json",
            "knowledge",
            "create-profile",
            "--interest",
            "50",
            "--strategy",
            "30",
            "--consensus",
            "30",
            "--rationale",
            "invalid total must roll back",
        ])
        .assert()
        .failure();
    let profiles_after_rejection = babata(&temp)
        .args(["--json", "knowledge", "list-profiles"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let profiles_after_rejection: Value =
        serde_json::from_slice(&profiles_after_rejection).unwrap();
    assert_eq!(profiles_after_rejection.as_array().unwrap().len(), 1);
    babata(&temp)
        .args([
            "--json",
            "knowledge",
            "create-profile",
            "--interest",
            "50",
            "--strategy",
            "30",
            "--consensus",
            "20",
            "--rationale",
            "new focus",
        ])
        .assert()
        .success();
    let rescored = babata(&temp)
        .args([
            "--json",
            "knowledge",
            "score",
            "--semantic",
            ingested["semantic_ids"][0].as_str().unwrap(),
            "--interest",
            "90",
            "--strategy",
            "80",
            "--consensus",
            "70",
            "--rationale",
            "reviewed under the new focus profile",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let rescored: Value = serde_json::from_slice(&rescored).unwrap();
    assert_eq!(rescored["weighted_score"], 8300);
    let score_history = babata(&temp)
        .args([
            "--json",
            "knowledge",
            "show-entry",
            "--semantic",
            ingested["semantic_ids"][0].as_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let score_history: Value = serde_json::from_slice(&score_history).unwrap();
    assert_eq!(score_history["scores"].as_array().unwrap().len(), 2);
    assert_ne!(
        score_history["scores"][0]["profile_id"],
        score_history["scores"][1]["profile_id"]
    );

    babata(&temp)
        .args([
            "--json",
            "knowledge",
            "review-suggestion",
            "--suggestion",
            suggestion,
            "--decision",
            "accept",
        ])
        .assert()
        .success();
    let first_party = babata(&temp)
        .args([
            "--json",
            "create",
            "--text",
            "我的独立感悟，不是来源资料 v2。",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let first_party: Value = serde_json::from_slice(&first_party).unwrap();
    let insight_definition = serde_json::json!({
        "title": "关于工具复用的个人感悟",
        "payload": {
            "kind": "insight",
            "maturity": "spark",
            "body": "我的独立感悟，不是来源资料 v2。"
        },
        "map_node_refs": ["foundation:consciousness"],
        "tags": ["个人感悟"],
        "dense_expressions": [],
        "relevance": {"interest": 90, "strategy": 70, "consensus": 20, "rationale": "刚形成的个人判断"},
        "relations": [{
            "kind": "reflects_on",
            "to_semantic_id": ingested["semantic_ids"][0].as_str().unwrap(),
            "evidence": "这条感悟由该知识候选触发"
        }]
    });
    let insight_path = temp.path().join("insight-definition.json");
    std::fs::write(&insight_path, insight_definition.to_string()).unwrap();

    let external_log_definition = serde_json::json!({
        "title": "不能伪装成第一方日志的外部资料",
        "payload": {
            "kind": "log",
            "scale": "realtime",
            "occurred_at": "2026-07-21T00:00:00Z",
            "body": "crawler engineering source with a concrete repository case"
        },
        "map_node_refs": ["foundation:time"],
        "tags": [],
        "dense_expressions": [],
        "relevance": {"interest": 1, "strategy": 1, "consensus": 1, "rationale": "negative boundary"},
        "relations": []
    });
    let external_log_path = temp.path().join("external-log-definition.json");
    std::fs::write(&external_log_path, external_log_definition.to_string()).unwrap();
    babata(&temp)
        .args([
            "--json",
            "knowledge",
            "register-first-party",
            "--item",
            item,
            "--revision",
            revision,
            "--definition",
            external_log_path.to_str().unwrap(),
        ])
        .assert()
        .failure();

    let mut mismatched_insight = insight_definition.clone();
    mismatched_insight["payload"]["body"] = Value::String("模型替用户改过的正文".to_owned());
    let mismatched_insight_path = temp.path().join("mismatched-insight-definition.json");
    std::fs::write(&mismatched_insight_path, mismatched_insight.to_string()).unwrap();
    babata(&temp)
        .args([
            "--json",
            "knowledge",
            "register-first-party",
            "--item",
            first_party["item_id"].as_str().unwrap(),
            "--revision",
            first_party["revision_id"].as_str().unwrap(),
            "--definition",
            mismatched_insight_path.to_str().unwrap(),
        ])
        .assert()
        .failure();

    let insight = babata(&temp)
        .args([
            "--json",
            "knowledge",
            "register-first-party",
            "--item",
            first_party["item_id"].as_str().unwrap(),
            "--revision",
            first_party["revision_id"].as_str().unwrap(),
            "--definition",
            insight_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let insight: Value = serde_json::from_slice(&insight).unwrap();
    assert_eq!(insight["kind"], "insight");
    assert_eq!(insight["realm"], "cognitive_trail");
    assert_eq!(insight["origin_kind"], "first_party");

    let log_c0 = babata(&temp)
        .args([
            "--json",
            "create",
            "--text",
            "今天完成 P6 语义核心首轮验证。",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let log_c0: Value = serde_json::from_slice(&log_c0).unwrap();
    let log_definition = serde_json::json!({
        "title": "P6 实时日志",
        "payload": {
            "kind": "log",
            "scale": "realtime",
            "occurred_at": "2026-07-21T00:00:00Z",
            "body": "今天完成 P6 语义核心首轮验证。"
        },
        "map_node_refs": ["foundation:time", "foundation:consciousness"],
        "tags": ["P6"],
        "dense_expressions": [],
        "relevance": {"interest": 85, "strategy": 80, "consensus": 30, "rationale": "当前阶段日志"},
        "relations": []
    });
    let log_path = temp.path().join("log-definition.json");
    std::fs::write(&log_path, log_definition.to_string()).unwrap();
    let log = babata(&temp)
        .args([
            "--json",
            "knowledge",
            "register-first-party",
            "--item",
            log_c0["item_id"].as_str().unwrap(),
            "--revision",
            log_c0["revision_id"].as_str().unwrap(),
            "--definition",
            log_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let log: Value = serde_json::from_slice(&log).unwrap();
    assert_eq!(log["kind"], "log");
    assert_eq!(log["realm"], "cognitive_trail");

    babata(&temp)
        .args([
            "--json",
            "knowledge",
            "review-suggestion",
            "--suggestion",
            suggestion,
            "--decision",
            "modify",
            "--first-party-item",
            first_party["item_id"].as_str().unwrap(),
            "--first-party-revision",
            first_party["revision_id"].as_str().unwrap(),
        ])
        .assert()
        .success();
    babata(&temp)
        .args([
            "--json",
            "knowledge",
            "review-suggestion",
            "--suggestion",
            suggestion,
            "--decision",
            "reject",
            "--reason",
            "later evidence contradicted it",
        ])
        .assert()
        .success();
    let reviewed = show();
    assert_eq!(reviewed["suggestion"]["review_state"], "rejected");
    assert_eq!(reviewed["reviews"].as_array().unwrap().len(), 3);
    assert!(
        !reviewed["suggestion"]["downstream_eligibility"]["eligible_uses"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value == "output_candidate")
    );
    assert!(
        reviewed["suggestion"]["downstream_eligibility"]["eligible_uses"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value == "search")
    );

    let mut reanalysis_package = package.clone();
    reanalysis_package["model"] = Value::String("fixture-reanalysis-model".to_owned());
    reanalysis_package["model_version"] = Value::String("2.0.0".to_owned());
    reanalysis_package["generated_at"] = Value::String("2026-07-21T01:00:00Z".to_owned());
    reanalysis_package["entries"][0]["title"] =
        Value::String("新增证据后的爬虫工程策略".to_owned());
    reanalysis_package["entries"][1]["title"] =
        Value::String("新增证据后的工具组合案例".to_owned());
    reanalysis_package["limitations"] = serde_json::json!(["second fixture analysis"]);
    let reanalysis_path = temp.path().join("semantic-reanalysis-package.json");
    std::fs::write(&reanalysis_path, reanalysis_package.to_string()).unwrap();
    let reanalysis_derivative = babata(&temp)
        .args([
            "--json",
            "process",
            "register",
            "--pipeline",
            "agent_import",
            "--revision",
            revision,
            "--kind",
            "structured_result",
            "--provider",
            "fixture",
            "--model",
            "fixture-reanalysis-model",
            "--tool-version",
            "2.0.0",
            "--input-sha256",
            input_sha,
            "--json-file",
            reanalysis_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let reanalysis_derivative: Value = serde_json::from_slice(&reanalysis_derivative).unwrap();
    let reanalysis = babata(&temp)
        .args([
            "--json",
            "knowledge",
            "ingest",
            "--derivative",
            reanalysis_derivative["derivative_id"].as_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let reanalysis: Value = serde_json::from_slice(&reanalysis).unwrap();
    assert_ne!(reanalysis["suggestion_id"], suggestion);
    assert_eq!(reanalysis["review_state"], "unreviewed");
    let reanalysis_snapshot = babata(&temp)
        .args([
            "--json",
            "knowledge",
            "show",
            "--suggestion",
            reanalysis["suggestion_id"].as_str().unwrap(),
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let reanalysis_snapshot: Value = serde_json::from_slice(&reanalysis_snapshot).unwrap();
    assert_eq!(
        reanalysis_snapshot["suggestion"]["review_state"],
        "unreviewed"
    );
    assert!(
        reanalysis_snapshot["suggestion"]["downstream_eligibility"]["eligible_uses"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value == "output_candidate")
    );

    let source = babata(&temp)
        .args([
            "--json",
            "knowledge",
            "review",
            "--item",
            item,
            "--revision",
            revision,
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let source: Value = serde_json::from_slice(&source).unwrap();
    assert_eq!(source["target"]["revisions"].as_array().unwrap().len(), 1);
}
