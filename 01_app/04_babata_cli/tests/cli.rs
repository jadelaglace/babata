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
fn capability_list_reports_processing_enabled_for_p5_register() {
    let temp = tempdir().unwrap();
    let output = babata(&temp)
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
    for provider in [
        "processing.local_extract",
        "processing.bailian_cli",
        "processing.bailian_api",
    ] {
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
