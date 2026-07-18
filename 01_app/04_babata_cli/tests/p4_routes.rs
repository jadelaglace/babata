use std::path::PathBuf;

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

fn output_json(temp: &tempfile::TempDir, args: &[&str], succeeds: bool) -> Value {
    let mut assertion = babata(temp).args(args).assert();
    assertion = if succeeds {
        assertion.success()
    } else {
        assertion.failure()
    };
    serde_json::from_slice(&assertion.get_output().stdout).unwrap()
}

#[test]
fn p4_capture_commands_remain_unavailable_before_activation() {
    let temp = tempdir().unwrap();
    let export = fixture("04_tests/03_fixtures/07_feishu/official-export.md");
    let cases = [
        vec!["--json", "capture", "candidate", "--path", &export],
        vec![
            "--json",
            "capture",
            "feishu-export",
            "--path",
            &export,
            "--locator",
            "https://example.test/doc",
            "--native-id",
            "fixture",
        ],
        vec!["--json", "capture", "bookmarks", "--path", &export],
        vec!["--json", "routes", "evaluate", "source.feishu"],
    ];
    for args in cases {
        let error = output_json(&temp, &args, false);
        assert_eq!(error["code"], "capability_unavailable");
    }
    assert!(!temp.path().join("01_raw/index/raw.sqlite").exists());
}

#[test]
fn p4_capabilities_are_not_promoted_by_scaffold_or_fixtures() {
    let temp = tempdir().unwrap();
    let capabilities = output_json(&temp, &["--json", "capabilities", "list"], true);
    assert!(capabilities.as_array().unwrap().iter().any(|capability| {
        capability["id"] == "capture.candidate" && capability["status"] == "unavailable"
    }));
    for source in [
        "source.feishu",
        "source.kimi",
        "source.browser_pages",
        "source.browser_bookmarks",
    ] {
        assert!(capabilities.as_array().unwrap().iter().any(|capability| {
            capability["id"] == source && capability["status"] == "disabled"
        }));
    }
}

#[test]
fn explicit_export_fallback_does_not_activate_a_provider_route() {
    let temp = tempdir().unwrap();
    let export = fixture("04_tests/03_fixtures/03_exports/sample-export.md");
    let outcome = output_json(
        &temp,
        &[
            "--json",
            "capture",
            "export",
            "--provider",
            "fixture",
            "--path",
            &export,
        ],
        true,
    );
    assert_eq!(outcome["status"], "ready");
    let capabilities = output_json(&temp, &["--json", "capabilities", "list"], true);
    assert!(capabilities.as_array().unwrap().iter().any(|capability| {
        capability["id"] == "source.feishu" && capability["status"] == "disabled"
    }));
}
