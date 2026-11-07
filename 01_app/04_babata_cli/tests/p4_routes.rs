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
fn collector_is_enabled_without_promoting_partially_proven_source_routes() {
    let temp = tempdir().unwrap();
    let capabilities = output_json(&temp, &["--json", "capabilities", "list"], true);
    assert!(capabilities.as_array().unwrap().iter().any(|capability| {
        capability["id"] == "capture.candidate" && capability["status"] == "unavailable"
    }));
    assert!(capabilities.as_array().unwrap().iter().any(|capability| {
        capability["id"] == "collector" && capability["status"] == "enabled"
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
    assert!(capabilities.as_array().unwrap().iter().any(|capability| {
        capability["id"] == "source.evernote" && capability["status"] == "enabled"
    }));
    assert!(capabilities.as_array().unwrap().iter().any(|capability| {
        capability["id"] == "source.onenote" && capability["status"] == "enabled"
    }));
    assert!(capabilities.as_array().unwrap().iter().any(|capability| {
        capability["id"] == "source.doubao" && capability["status"] == "enabled"
    }));
    assert!(capabilities.as_array().unwrap().iter().any(|capability| {
        capability["id"] == "source.youtube" && capability["status"] == "enabled"
    }));
    assert!(capabilities.as_array().unwrap().iter().any(|capability| {
        capability["id"] == "source.yitang" && capability["status"] == "enabled"
    }));
}

#[test]
fn disabled_route_cannot_start_a_collector_session() {
    let temp = tempdir().unwrap();
    let error = output_json(
        &temp,
        &[
            "--json",
            "collector",
            "start",
            "--route",
            "source.kimi",
            "--source",
            "recent:1",
            "--scope",
            "one",
            "--authorisation",
            "fixture",
        ],
        false,
    );
    assert_eq!(error["code"], "capability_unavailable");
}

#[test]
fn qianwen_selected_native_rows_save_and_require_new_handoff_for_recollection() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("handoff.json");
    let id = "0123456789abcdef0123456789abcdef";
    let mut h = serde_json::json!({
        "protocol_version": "1", "provider": "qianwen", "conversation_id": id,
        "title": "fixture", "source_url": format!("https://www.qianwen.com/chat/{id}"),
        "have_next_page": false,
        "messages": [{"session_id": id, "req_id": "kept", "pos": "2", "event": "complete", "request_messages": [{"content": "retained long native content ".repeat(4096)}]}],
        "original_keys": ["food|1|complete", "kept|2|complete"], "excluded_keys": ["food|1|complete"], "excluded_fields": [], "assets": []
    });
    std::fs::write(&path, h.to_string()).unwrap();
    let path = path.to_str().unwrap();
    let session = output_json(
        &temp,
        &[
            "--json",
            "collector",
            "start",
            "--route",
            "source.qianwen",
            "--source",
            id,
            "--scope",
            "fixture",
            "--authorisation",
            "fixture",
            "--acquisition-handoff",
            path,
        ],
        true,
    );
    let session_id = session["session_id"].as_str().unwrap();
    let candidate_id = format!("qianwen_{id}");
    let saved = output_json(
        &temp,
        &[
            "--json",
            "collector",
            "select",
            "--session",
            session_id,
            "--candidate",
            &candidate_id,
            "--scope",
            "fixture",
            "--authorisation",
            "fixture",
            "--attachments",
            "--confirm",
        ],
        true,
    );
    assert_eq!(saved[0]["state"], "saved");
    let item = saved[0]["item_id"].as_str().unwrap();
    let missing = output_json(
        &temp,
        &["--json", "collector", "recollect", "--item", item],
        true,
    );
    assert_eq!(missing["state"], "inaccessible");
    let unchanged = output_json(
        &temp,
        &[
            "--json",
            "collector",
            "recollect",
            "--item",
            item,
            "--acquisition-handoff",
            path,
        ],
        true,
    );
    assert_eq!(unchanged["state"], "unchanged");
    assert!(unchanged["new_revision_id"].is_null());
    h["messages"][0]["request_messages"][0]["content"] = "updated source".into();
    std::fs::write(path, h.to_string()).unwrap();
    let changed = output_json(
        &temp,
        &[
            "--json",
            "collector",
            "recollect",
            "--item",
            item,
            "--acquisition-handoff",
            path,
        ],
        true,
    );
    assert_eq!(changed["state"], "changed");
    assert!(changed["new_revision_id"].is_string());
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
