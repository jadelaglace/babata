use assert_cmd::Command;
use serde_json::Value;
use tempfile::tempdir;

fn babata(temp: &tempfile::TempDir) -> Command {
    let mut command = Command::cargo_bin("babata").unwrap();
    command.env("BABATA_DATA_HOME", temp.path());
    command
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
fn process_register_and_retry_create_separate_runs() {
    let temp = tempdir().unwrap();
    // capture first to get a real revision id, then register against it
    let capture = babata(&temp)
        .args([
            "--json",
            "capture",
            "text",
            "--provider",
            "fixture",
            "--text",
            "source text for cleaning",
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let capture: Value = serde_json::from_slice(&capture).unwrap();
    let revision = capture["revision_id"].as_str().unwrap();
    let sha = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

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
            sha,
            "--text",
            "first summary",
            "--model",
            "qwen-plus",
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
            sha,
            "--text",
            "retry summary",
            "--retry-of",
            &run_id,
        ])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let second: Value = serde_json::from_slice(&second).unwrap();
    assert_ne!(first["run_id"], second["run_id"]);

    let runs = babata(&temp)
        .args(["--json", "process", "list-runs", "--revision", revision])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let runs: Value = serde_json::from_slice(&runs).unwrap();
    assert_eq!(runs.as_array().unwrap().len(), 2);
    assert_eq!(runs[1]["attempt"], 2);
    assert_eq!(runs[1]["retry_of_run_id"], run_id);
}
