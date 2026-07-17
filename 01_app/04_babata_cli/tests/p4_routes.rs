use std::{fs, path::PathBuf};

use assert_cmd::Command;
use serde_json::{Value, json};
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

fn output_json(temp: &tempfile::TempDir, args: &[&str]) -> Value {
    let output = babata(temp)
        .args(args)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&output).unwrap()
}

#[test]
fn p4_imports_feishu_export_bookmarks_and_browser_candidate() {
    let temp = tempdir().unwrap();
    let feishu = fixture("04_tests/03_fixtures/07_feishu/official-export.md");
    let attachment = fixture("04_tests/03_fixtures/07_feishu/attachment.txt");
    let first = output_json(
        &temp,
        &[
            "--json",
            "capture",
            "feishu-export",
            "--path",
            &feishu,
            "--locator",
            "https://example.feishu.cn/docx/fixture-token",
            "--native-id",
            "fixture-token",
            "--context",
            "wiki/engineering",
            "--attachment",
            &attachment,
        ],
    );
    let second = output_json(
        &temp,
        &[
            "--json",
            "capture",
            "feishu-export",
            "--path",
            &feishu,
            "--locator",
            "https://example.feishu.cn/docx/fixture-token",
            "--native-id",
            "fixture-token",
            "--context",
            "wiki/engineering",
            "--attachment",
            &attachment,
        ],
    );
    assert_eq!(first["item_id"], second["item_id"]);
    assert_eq!(first["revision_id"], second["duplicate_of"]);
    assert_eq!(first["asset_ids"].as_array().unwrap().len(), 2);

    let bookmarks = fixture("04_tests/03_fixtures/08_browser/bookmarks.html");
    let imported = output_json(
        &temp,
        &["--json", "capture", "bookmarks", "--path", &bookmarks],
    );
    assert_eq!(imported.as_array().unwrap().len(), 2);
    let reimported = output_json(
        &temp,
        &["--json", "capture", "bookmarks", "--path", &bookmarks],
    );
    for (first, second) in imported
        .as_array()
        .unwrap()
        .iter()
        .zip(reimported.as_array().unwrap())
    {
        assert_eq!(first["item_id"], second["item_id"]);
        assert_eq!(first["revision_id"], second["duplicate_of"]);
    }

    let candidate_path = temp.path().join("browser-candidate.json");
    fs::write(
        &candidate_path,
        serde_json::to_vec(&json!({
            "protocolVersion": "1",
            "routeId": "source.browser",
            "sourceReference": "https://example.test/clipped",
            "contentType": "web_page",
            "payloadSha256": "ac5f5488e223de843ea29b0139ecaf43d9e8da5eb7d5f343cbc135f9f191cc3b",
            "metadata": {"title": "Clipped page"},
            "payload": {"kind": "text", "text": "A clipped page"}
        }))
        .unwrap(),
    )
    .unwrap();
    let candidate = output_json(
        &temp,
        &[
            "--json",
            "capture",
            "candidate",
            "--path",
            &candidate_path.to_string_lossy(),
        ],
    );
    assert_eq!(candidate["status"], "ready");

    let capabilities = output_json(&temp, &["--json", "capabilities", "list"]);
    assert!(capabilities.as_array().unwrap().iter().any(|capability| {
        capability["id"] == "capture.candidate" && capability["status"] == "enabled"
    }));
    assert!(capabilities.as_array().unwrap().iter().any(|capability| {
        capability["id"] == "source.feishu" && capability["status"] == "disabled"
    }));
    assert!(temp.path().join("01_raw/index/raw.sqlite").exists());
}

#[test]
fn authorised_reimport_records_offline_coverage_without_enabling_the_route() {
    let temp = tempdir().unwrap();
    let fixture_export = fixture("04_tests/03_fixtures/07_feishu/official-export.md");
    let live_export = temp.path().join("authorised-export.md");
    fs::copy(fixture_export, &live_export).unwrap();
    let feishu = live_export.to_string_lossy().into_owned();
    let attachment = fixture("04_tests/03_fixtures/07_feishu/attachment.txt");
    let mut feishu_outcomes = Vec::new();
    for attempt in 0..2 {
        if attempt == 1 {
            fs::write(
                &live_export,
                "# Project Compass\n\nAn updated authorised export.",
            )
            .unwrap();
        }
        feishu_outcomes.push(output_json(
            &temp,
            &[
                "--json",
                "capture",
                "feishu-export",
                "--path",
                &feishu,
                "--locator",
                "https://example.feishu.cn/docx/authorised-test",
                "--native-id",
                "authorised-test",
                "--attachment",
                &attachment,
                "--authorized-test",
                "operator-attestation-for-test",
            ],
        ));
    }
    assert!(feishu_outcomes[1]["duplicate_of"].is_null());
    assert_eq!(feishu_outcomes[1]["reimported"], true);
    let coverage = output_json(&temp, &["--json", "routes", "evaluate", "source.feishu"]);
    assert_eq!(coverage["metadata"], true);
    assert_eq!(coverage["attachments"], true);
    assert_eq!(coverage["revisions"], true);
    let route = output_json(&temp, &["--json", "routes", "show", "source.feishu"]);
    assert_eq!(route["status"], "disabled");

    let bookmarks = fixture("04_tests/03_fixtures/08_browser/bookmarks.html");
    for _ in 0..2 {
        output_json(
            &temp,
            &[
                "--json",
                "capture",
                "bookmarks",
                "--path",
                &bookmarks,
                "--authorized-test",
                "operator-attestation-for-test",
            ],
        );
    }
    let browser_coverage = output_json(&temp, &["--json", "routes", "evaluate", "source.browser"]);
    assert_eq!(browser_coverage["metadata"], true);
    assert_eq!(browser_coverage["attachments"], false);
    assert_eq!(browser_coverage["revisions"], true);
    let browser = output_json(&temp, &["--json", "routes", "show", "source.browser"]);
    assert_eq!(browser["status"], "disabled");

    let capabilities = output_json(&temp, &["--json", "capabilities", "list"]);
    assert!(capabilities.as_array().unwrap().iter().any(|capability| {
        capability["id"] == "source.feishu" && capability["status"] == "disabled"
    }));
    assert!(capabilities.as_array().unwrap().iter().any(|capability| {
        capability["id"] == "source.browser" && capability["status"] == "disabled"
    }));
}

#[test]
fn candidate_with_wrong_payload_hash_is_rejected_before_writing() {
    let temp = tempdir().unwrap();
    let candidate_path = temp.path().join("bad-candidate.json");
    fs::write(
        &candidate_path,
        serde_json::to_vec(&json!({
            "protocolVersion": "1",
            "routeId": "source.browser",
            "sourceReference": "https://example.test/bad",
            "contentType": "web_page",
            "payloadSha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "metadata": {},
            "payload": {"kind": "text", "text": "A clipped page"}
        }))
        .unwrap(),
    )
    .unwrap();
    let output = babata(&temp)
        .args([
            "--json",
            "capture",
            "candidate",
            "--path",
            &candidate_path.to_string_lossy(),
        ])
        .assert()
        .failure()
        .get_output()
        .stdout
        .clone();
    let error: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(error["code"], "integrity_failed");
    assert_eq!(
        fs::read_dir(temp.path().join("01_raw/assets"))
            .unwrap()
            .count(),
        1
    );
    assert_eq!(
        fs::read_dir(temp.path().join("04_runtime/asset-journal"))
            .unwrap()
            .count(),
        0
    );
}
