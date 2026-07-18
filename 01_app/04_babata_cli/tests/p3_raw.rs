use std::{fs, path::PathBuf};

use assert_cmd::Command;
use babata_infrastructure::test_support::{
    capture_operation_snapshot, inject_graph_failure, inject_post_ready_readback_failure,
    inject_ready_failure,
};
use serde_json::Value;
use tempfile::tempdir;

fn babata(temp: &tempfile::TempDir) -> Command {
    let mut command = Command::cargo_bin("babata").unwrap();
    command.env("BABATA_DATA_HOME", temp.path());
    command
}

fn fixture(relative: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
        .canonicalize()
        .unwrap()
        .to_string_lossy()
        .into_owned()
}

fn json_success(temp: &tempfile::TempDir, args: &[&str]) -> Value {
    let output = babata(temp)
        .args(args)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&output).unwrap()
}

fn json_file_capture(
    temp: &tempfile::TempDir,
    input: &std::path::Path,
    fault: Option<&str>,
    succeeds: bool,
) -> Value {
    let mut command = babata(temp);
    command.args([
        "--json",
        "capture",
        "file",
        "--provider",
        "fixture-fault",
        "--path",
        &input.to_string_lossy(),
    ]);
    if let Some(fault) = fault {
        command.env("BABATA_TEST_ASSET_FAULT", fault);
    }
    let assertion = command.assert();
    let output = if succeeds {
        assertion.success()
    } else {
        assertion.failure()
    }
    .get_output()
    .stdout
    .clone();
    serde_json::from_slice(&output).unwrap()
}

fn recovery_counts(temp: &tempfile::TempDir) -> (usize, usize) {
    (
        fs::read_dir(temp.path().join("04_runtime/asset-journal"))
            .unwrap()
            .count(),
        fs::read_dir(temp.path().join("01_raw/quarantine/orphans"))
            .unwrap()
            .count(),
    )
}

fn install_ready_failure(temp: &tempfile::TempDir) {
    inject_ready_failure(temp.path()).unwrap();
}

fn files_under(path: &std::path::Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            files.extend(files_under(&path));
        } else {
            files.push(path);
        }
    }
    files
}

#[test]
#[allow(clippy::too_many_lines)]
fn raw_cli_flow_keeps_assets_and_first_party_lineage() {
    let temp = tempdir().unwrap();
    let first = json_success(
        &temp,
        &[
            "--json",
            "capture",
            "text",
            "--provider",
            "fixture",
            "--text",
            "raw wording",
            "--context",
            "p3",
        ],
    );
    assert!(first["operation_id"].as_str().unwrap().starts_with("op_"));
    assert_eq!(first["record"]["provider"], "fixture");
    assert_eq!(first["record"]["collections"][0]["native_id"], "p3");
    assert_eq!(first["record"]["revisions"][0]["state"], "ready");
    assert_eq!(first["record"]["revisions"][0]["raw_text"], "raw wording");
    let second = json_success(
        &temp,
        &[
            "--json",
            "capture",
            "text",
            "--provider",
            "fixture",
            "--text",
            "raw wording",
            "--context",
            "p3",
        ],
    );
    assert_eq!(first["item_id"], second["item_id"]);
    assert_eq!(first["revision_id"], second["duplicate_of"]);

    let file_path = fixture("04_tests/03_fixtures/02_files/sample.txt");
    let export_path = fixture("04_tests/03_fixtures/03_exports/sample-export.md");
    let file = json_success(
        &temp,
        &[
            "--json",
            "capture",
            "file",
            "--provider",
            "fixture",
            "--path",
            &file_path,
        ],
    );
    let export = json_success(
        &temp,
        &[
            "--json",
            "capture",
            "export",
            "--provider",
            "fixture",
            "--path",
            &export_path,
        ],
    );
    assert_eq!(file["asset_ids"].as_array().unwrap().len(), 1);
    assert_eq!(export["asset_ids"].as_array().unwrap().len(), 1);
    assert_eq!(file["record"]["assets"][0]["role"], "original");
    assert_eq!(file["record"]["assets"][0]["state"], "ready");
    assert_eq!(export["record"]["assets"][0]["role"], "export");
    assert_eq!(export["record"]["assets"][0]["state"], "ready");
    for outcome in [&file, &export] {
        let asset = &outcome["record"]["assets"][0];
        assert!(
            asset["logical_path"]
                .as_str()
                .unwrap()
                .ends_with(asset["sha256"].as_str().unwrap())
        );
    }

    let authored = json_success(&temp, &["--json", "create", "--text", "first version"]);
    let authored_revision = authored["revision_id"].as_str().unwrap();
    let revised = json_success(
        &temp,
        &[
            "--json",
            "revise",
            "--parent",
            authored_revision,
            "--text",
            "second version",
            "--metadata-json",
            r#"{"stage":"revision"}"#,
        ],
    );
    let annotation = json_success(
        &temp,
        &[
            "--json",
            "annotate",
            "--target",
            authored_revision,
            "--text",
            "my annotation",
        ],
    );
    assert_eq!(authored["item_id"], revised["item_id"]);
    assert_ne!(authored["item_id"], annotation["item_id"]);
    assert_eq!(revised["record"]["source_kind"], "first_party");
    assert_eq!(revised["record"]["revisions"].as_array().unwrap().len(), 2);
    assert_eq!(
        revised["record"]["revisions"][0]["raw_text"],
        "first version"
    );
    assert_eq!(
        revised["record"]["revisions"][1]["raw_text"],
        "second version"
    );
    assert_eq!(
        revised["record"]["revisions"][1]["metadata"]["stage"],
        "revision"
    );
    assert_eq!(annotation["record"]["source_kind"], "first_party");
    assert_eq!(annotation["record"]["relations"][0]["kind"], "annotates");
    assert_eq!(
        annotation["record"]["relations"][0]["to_revision_id"],
        authored["revision_id"]
    );

    assert!(temp.path().join("01_raw/index/raw.sqlite").exists());
    let assets = files_under(&temp.path().join("01_raw/assets"));
    assert_eq!(assets.len(), 2);
    let contents = assets
        .into_iter()
        .map(fs::read)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    assert!(contents.contains(&fs::read(file_path).unwrap()));
    assert!(contents.contains(&fs::read(export_path).unwrap()));
    assert_eq!(
        fs::read_dir(temp.path().join("04_runtime/asset-journal"))
            .unwrap()
            .count(),
        0
    );
    assert_eq!(
        fs::read_dir(temp.path().join("01_raw/quarantine/orphans"))
            .unwrap()
            .count(),
        0
    );

    let status = json_success(&temp, &["--json", "data", "status"]);
    assert_eq!(status["raw_schema_version"], 4);
    assert_eq!(status["pending_journals"], 0);
    assert_eq!(status["orphans"], 0);
    assert_eq!(status["quarantined_revisions"], 0);
    assert_eq!(status["pending_operations"], 0);
    assert_eq!(status["quarantined_operations"], 0);
}

#[test]
fn json_error_does_not_expose_the_input_absolute_path() {
    let temp = tempdir().unwrap();
    let missing = temp.path().join("private-input.txt");
    let missing_text = missing.to_string_lossy().into_owned();
    let output = babata(&temp)
        .args([
            "--json",
            "capture",
            "file",
            "--provider",
            "fixture",
            "--path",
            &missing_text,
        ])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let error: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(error["code"], "io_failed");
    assert!(error["operation_id"].as_str().unwrap().starts_with("op_"));
    assert!(!error["message"].as_str().unwrap().contains(&missing_text));
    assert_eq!(
        fs::read_dir(temp.path().join("04_runtime/asset-journal"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn reimport_preserves_each_capture_provenance() {
    let temp = tempdir().unwrap();
    let first = json_success(
        &temp,
        &[
            "--json",
            "capture",
            "text",
            "--provider",
            "fixture",
            "--text",
            "v1",
            "--identity",
            "shared",
            "--native-id",
            "native-v1",
            "--locator",
            "https://first",
            "--source-published-at",
            "2026-01-01T00:00:00Z",
            "--metadata-json",
            r#"{"capture":"first"}"#,
            "--context",
            "first",
        ],
    );
    let second = json_success(
        &temp,
        &[
            "--json",
            "capture",
            "text",
            "--provider",
            "fixture",
            "--text",
            "v2",
            "--identity",
            "shared",
            "--native-id",
            "native-v2",
            "--locator",
            "https://second",
            "--source-published-at",
            "2026-02-02T00:00:00Z",
            "--metadata-json",
            r#"{"capture":"second"}"#,
            "--context",
            "second",
        ],
    );
    assert_eq!(first["item_id"], second["item_id"]);
    assert_eq!(second["record"]["source_locator"], "https://first");
    let revisions = second["record"]["revisions"].as_array().unwrap();
    assert_eq!(revisions.len(), 2);
    assert_eq!(revisions[0]["raw_text"], "v1");
    assert_eq!(revisions[1]["raw_text"], "v2");
    assert_eq!(revisions[0]["provenance"]["source_native_id"], "native-v1");
    assert_eq!(revisions[1]["provenance"]["source_native_id"], "native-v2");
    assert_eq!(
        revisions[0]["provenance"]["source_locator"],
        "https://first"
    );
    assert_eq!(
        revisions[1]["provenance"]["source_locator"],
        "https://second"
    );
    assert_eq!(
        revisions[0]["provenance"]["source_published_at"],
        "2026-01-01T00:00:00Z"
    );
    assert_eq!(
        revisions[1]["provenance"]["source_published_at"],
        "2026-02-02T00:00:00Z"
    );
    assert_eq!(revisions[0]["provenance"]["metadata"]["capture"], "first");
    assert_eq!(revisions[1]["provenance"]["metadata"]["capture"], "second");
    assert_eq!(revisions[0]["metadata"]["capture"], "first");
    assert_eq!(revisions[1]["metadata"]["capture"], "second");
    assert_ne!(
        revisions[0]["provenance"]["operation_id"],
        revisions[1]["provenance"]["operation_id"]
    );
    assert_eq!(second["record"]["collections"].as_array().unwrap().len(), 2);
}

#[test]
fn reimport_keeps_each_immutable_asset() {
    let temp = tempdir().unwrap();
    let first_file = temp.path().join("reimport-v1.txt");
    let second_file = temp.path().join("reimport-v2.txt");
    fs::write(&first_file, "asset v1").unwrap();
    fs::write(&second_file, "asset v2").unwrap();
    json_success(
        &temp,
        &[
            "--json",
            "capture",
            "file",
            "--provider",
            "fixture-file",
            "--path",
            &first_file.to_string_lossy(),
            "--identity",
            "shared-file",
            "--locator",
            "file://first",
        ],
    );
    let file_reimport = json_success(
        &temp,
        &[
            "--json",
            "capture",
            "file",
            "--provider",
            "fixture-file",
            "--path",
            &second_file.to_string_lossy(),
            "--identity",
            "shared-file",
            "--locator",
            "file://second",
        ],
    );
    let stored_assets = file_reimport["record"]["assets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|asset| fs::read(temp.path().join(asset["logical_path"].as_str().unwrap())).unwrap())
        .collect::<Vec<_>>();
    assert_eq!(stored_assets.len(), 2);
    assert!(stored_assets.contains(&b"asset v1".to_vec()));
    assert!(stored_assets.contains(&b"asset v2".to_vec()));
}

#[test]
fn post_ready_readback_failure_reports_a_committed_capture() {
    let temp = tempdir().unwrap();
    json_success(
        &temp,
        &[
            "--json",
            "capture",
            "text",
            "--provider",
            "fixture",
            "--text",
            "seed",
        ],
    );
    inject_post_ready_readback_failure(temp.path()).unwrap();
    let input = temp.path().join("post-ready.txt");
    fs::write(&input, "durable bytes").unwrap();
    let output = json_success(
        &temp,
        &[
            "--json",
            "capture",
            "file",
            "--provider",
            "fixture-post-ready",
            "--path",
            &input.to_string_lossy(),
        ],
    );
    assert_eq!(output["status"], "ready");
    assert!(output["record"].is_null());
    assert!(
        output["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| warning.as_str().unwrap().contains("read-back failed"))
    );
    let operation_id = output["operation_id"].as_str().unwrap();
    let revision_id = output["revision_id"].as_str().unwrap();
    let snapshot = capture_operation_snapshot(temp.path(), operation_id)
        .unwrap()
        .unwrap();
    assert_eq!(snapshot.revision_id, revision_id);
    assert_eq!(snapshot.revision_state, "ready");
    assert_eq!(snapshot.asset_state.as_deref(), Some("ready"));
    assert_eq!(snapshot.operation_state, "ready");
    assert_eq!(
        fs::read(temp.path().join(snapshot.logical_path.unwrap())).unwrap(),
        b"durable bytes"
    );
    assert_eq!(
        fs::read_dir(temp.path().join("04_runtime/asset-journal"))
            .unwrap()
            .count(),
        0
    );
    assert_eq!(
        fs::read_dir(temp.path().join("01_raw/quarantine/orphans"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn graph_failure_cli_rolls_back_database_and_staging() {
    let temp = tempdir().unwrap();
    json_success(
        &temp,
        &[
            "--json",
            "capture",
            "text",
            "--provider",
            "fixture",
            "--text",
            "seed",
        ],
    );
    inject_graph_failure(temp.path()).unwrap();
    let input = temp.path().join("graph.txt");
    fs::write(&input, "graph bytes").unwrap();
    let error = json_file_capture(&temp, &input, None, false);
    let operation_id = error["operation_id"].as_str().unwrap();
    assert_eq!(error["code"], "io_failed");
    assert!(
        capture_operation_snapshot(temp.path(), operation_id)
            .unwrap()
            .is_none()
    );
    assert_eq!(recovery_counts(&temp), (0, 0));
    assert!(files_under(&temp.path().join("01_raw/assets")).is_empty());
}

#[test]
fn finalization_failure_cli_keeps_correlated_quarantine() {
    let temp = tempdir().unwrap();
    let input = temp.path().join("finalize.txt");
    fs::write(&input, "finalize bytes").unwrap();
    let error = json_file_capture(&temp, &input, Some("finalize"), false);
    let operation_id = error["operation_id"].as_str().unwrap();
    let snapshot = capture_operation_snapshot(temp.path(), operation_id)
        .unwrap()
        .unwrap();
    assert_eq!(error["code"], "io_failed");
    assert_eq!(snapshot.operation_state, "quarantined");
    assert_eq!(snapshot.revision_state, "quarantined");
    assert_eq!(snapshot.asset_state.as_deref(), Some("quarantined"));
    assert!(!temp.path().join(snapshot.logical_path.unwrap()).exists());
    assert_eq!(recovery_counts(&temp), (1, 0));
}

#[test]
fn verification_failure_cli_preserves_final_bytes_and_orphan() {
    let temp = tempdir().unwrap();
    let input = temp.path().join("verify.txt");
    fs::write(&input, "verify bytes").unwrap();
    let error = json_file_capture(&temp, &input, Some("verify"), false);
    let operation_id = error["operation_id"].as_str().unwrap();
    let snapshot = capture_operation_snapshot(temp.path(), operation_id)
        .unwrap()
        .unwrap();
    assert_eq!(error["code"], "integrity_failed");
    assert_eq!(snapshot.failure_code.as_deref(), Some("integrity_failed"));
    assert_eq!(snapshot.operation_state, "quarantined");
    assert_eq!(snapshot.revision_state, "quarantined");
    assert_eq!(snapshot.asset_state.as_deref(), Some("quarantined"));
    assert_eq!(
        fs::read(temp.path().join(snapshot.logical_path.unwrap())).unwrap(),
        b"verify bytes"
    );
    assert_eq!(recovery_counts(&temp), (1, 1));
}

#[test]
fn ready_failure_cli_preserves_final_bytes_and_operation_id() {
    let temp = tempdir().unwrap();
    json_success(
        &temp,
        &[
            "--json",
            "capture",
            "text",
            "--provider",
            "fixture",
            "--text",
            "seed",
        ],
    );
    install_ready_failure(&temp);
    let input = temp.path().join("ready.txt");
    fs::write(&input, "ready failure bytes").unwrap();
    let error = json_file_capture(&temp, &input, None, false);
    let operation_id = error["operation_id"].as_str().unwrap();
    let snapshot = capture_operation_snapshot(temp.path(), operation_id)
        .unwrap()
        .unwrap();
    assert_eq!(error["code"], "io_failed");
    assert_eq!(snapshot.failure_code.as_deref(), Some("io_failed"));
    assert_eq!(snapshot.operation_state, "quarantined");
    assert_eq!(snapshot.revision_state, "quarantined");
    assert_eq!(snapshot.asset_state.as_deref(), Some("quarantined"));
    assert_eq!(
        fs::read(temp.path().join(snapshot.logical_path.unwrap())).unwrap(),
        b"ready failure bytes"
    );
    assert_eq!(recovery_counts(&temp), (1, 1));
}

#[test]
fn cleanup_failure_cli_returns_ready_with_pending_journal() {
    let temp = tempdir().unwrap();
    let input = temp.path().join("cleanup.txt");
    fs::write(&input, "cleanup bytes").unwrap();
    let outcome = json_file_capture(&temp, &input, Some("cleanup"), true);
    let operation_id = outcome["operation_id"].as_str().unwrap();
    let snapshot = capture_operation_snapshot(temp.path(), operation_id)
        .unwrap()
        .unwrap();
    assert_eq!(outcome["status"], "ready");
    assert!(outcome["record"].is_object());
    assert!(
        outcome["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| warning.as_str().unwrap().contains("journal cleanup"))
    );
    assert_eq!(snapshot.operation_state, "ready");
    assert_eq!(snapshot.revision_state, "ready");
    assert_eq!(snapshot.asset_state.as_deref(), Some("ready"));
    assert_eq!(
        fs::read(temp.path().join(snapshot.logical_path.unwrap())).unwrap(),
        b"cleanup bytes"
    );
    assert_eq!(recovery_counts(&temp), (1, 0));
}

#[test]
fn pathless_ready_failures_keep_correlated_operation_diagnostics() {
    for command_kind in ["text", "create", "revise", "annotate"] {
        let temp = tempdir().unwrap();
        let seed = json_success(&temp, &["--json", "create", "--text", "seed"]);
        install_ready_failure(&temp);
        let target = seed["revision_id"].as_str().unwrap().to_owned();
        let args = match command_kind {
            "text" => vec![
                "--json".to_owned(),
                "capture".to_owned(),
                "text".to_owned(),
                "--provider".to_owned(),
                "fixture".to_owned(),
                "--text".to_owned(),
                "failed text".to_owned(),
            ],
            "create" => vec![
                "--json".to_owned(),
                "create".to_owned(),
                "--text".to_owned(),
                "failed create".to_owned(),
            ],
            "revise" => vec![
                "--json".to_owned(),
                "revise".to_owned(),
                "--parent".to_owned(),
                target,
                "--text".to_owned(),
                "failed revise".to_owned(),
            ],
            "annotate" => vec![
                "--json".to_owned(),
                "annotate".to_owned(),
                "--target".to_owned(),
                target,
                "--text".to_owned(),
                "failed annotate".to_owned(),
            ],
            _ => unreachable!(),
        };
        let bytes = babata(&temp)
            .args(args)
            .assert()
            .failure()
            .get_output()
            .stdout
            .clone();
        let error: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(error["code"], "io_failed", "{command_kind}");
        let operation_id = error["operation_id"].as_str().unwrap();
        let snapshot = capture_operation_snapshot(temp.path(), operation_id)
            .unwrap()
            .unwrap();
        assert_eq!(snapshot.operation_state, "quarantined", "{command_kind}");
        assert_eq!(snapshot.revision_state, "quarantined", "{command_kind}");
        assert_eq!(
            snapshot.failure_code.as_deref(),
            Some("io_failed"),
            "{command_kind}"
        );
        let journal: Value = serde_json::from_slice(
            &fs::read(
                temp.path()
                    .join("04_runtime/asset-journal")
                    .join(format!("{operation_id}.json")),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(journal["operation_id"], operation_id, "{command_kind}");
        assert_eq!(
            journal["revision_id"], snapshot.revision_id,
            "{command_kind}"
        );
        assert_eq!(journal["state"], "recovery_required", "{command_kind}");
        let status = json_success(&temp, &["--json", "data", "status"]);
        assert_eq!(status["pending_journals"], 1, "{command_kind}");
        assert_eq!(status["orphans"], 0, "{command_kind}");
        assert_eq!(status["quarantined_revisions"], 1, "{command_kind}");
        assert_eq!(status["quarantined_operations"], 1, "{command_kind}");
    }
}
