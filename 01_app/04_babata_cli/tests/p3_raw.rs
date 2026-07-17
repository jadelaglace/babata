use std::{fs, path::PathBuf};

use assert_cmd::Command;
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
    assert!(!error["message"].as_str().unwrap().contains(&missing_text));
}
