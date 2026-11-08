use assert_cmd::Command;
use babata_application::ports::RawRepositoryPort;
use babata_domain::{ItemId, RawState, Sha256};
use babata_infrastructure::{open_raw_database, paths::DataPaths};
use serde_json::{Value, json};

fn run(temp: &tempfile::TempDir, args: &[&str], success: bool) -> Value {
    let mut command = Command::cargo_bin("babata").unwrap();
    command
        .env("BABATA_DATA_HOME", temp.path())
        .arg("--json")
        .args(args);
    let assertion = if success {
        command.assert().success()
    } else {
        command.assert().failure()
    };
    serde_json::from_slice(&assertion.get_output().stdout).unwrap()
}

fn handoff(id: &str) -> Value {
    json!({"protocol_version":"1","provider":"yuanbao","conversation_id":id,
        "agent_id":"fixture-agent","title":"fixture","has_more":false,"assets":[],
        "messages":[{"id":format!("{id}_1"),"conversationId":id,"speaker":"human",
            "createTime":1_794_140_924,"speechesV2":[{"content":[{"type":"text","msg":"native text ".repeat(8192)}]}]}]})
}

#[test]
#[allow(clippy::too_many_lines)]
fn yuanbao_selection_large_native_payload_media_and_recollection_use_one_writer() {
    let temp = tempfile::tempdir().unwrap();
    let first = temp.path().join("first.json");
    let second = temp.path().join("second.json");
    let media = temp.path().join("media.bin");
    std::fs::write(&media, b"fixture media bytes").unwrap();
    let mut h = handoff("fixture-chat");
    let url = "https://hunyuan.tencent.com/api/resource/download?resourceId=fixture";
    h["messages"][0]["speechesV2"][0]["content"]
        .as_array_mut()
        .unwrap()
        .push(json!({"type":"image","url":url}));
    h["assets"] = json!([{"url":url,"path":media.to_str().unwrap(),"sha256":Sha256::of_bytes(b"fixture media bytes").as_str(),"bytes":19}]);
    std::fs::write(&first, h.to_string()).unwrap();
    std::fs::write(&second, handoff("other-chat").to_string()).unwrap();
    let session = run(
        &temp,
        &[
            "collector",
            "start",
            "--route",
            "source.yuanbao",
            "--source",
            "conversations:fixture-chat,other-chat",
            "--scope",
            "two fixtures",
            "--authorisation",
            "fixture",
            "--acquisition-handoff",
            first.to_str().unwrap(),
            "--acquisition-handoff",
            second.to_str().unwrap(),
        ],
        true,
    );
    let session_id = session["session_id"].as_str().unwrap();
    let candidates = run(
        &temp,
        &["collector", "candidates", "--session", session_id],
        true,
    );
    assert_eq!(candidates.as_array().unwrap().len(), 2);
    let saved = run(
        &temp,
        &[
            "collector",
            "select",
            "--session",
            session_id,
            "--candidate",
            "yuanbao_fixture-chat",
            "--scope",
            "one fixture",
            "--authorisation",
            "fixture",
            "--attachments",
            "--confirm",
        ],
        true,
    );
    let selected = saved
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["candidate_id"] == "yuanbao_fixture-chat")
        .unwrap();
    assert_eq!(selected["state"], "saved");
    assert!(
        saved
            .as_array()
            .unwrap()
            .iter()
            .filter(|r| r["candidate_id"] != "yuanbao_fixture-chat")
            .all(|r| r["item_id"].is_null())
    );
    let item = selected["item_id"].as_str().unwrap();
    let paths = DataPaths::new(temp.path().to_path_buf());
    let repository = open_raw_database(&paths, 5000).unwrap();
    let detail = repository
        .load_detail(&ItemId::parse(item).unwrap())
        .unwrap();
    assert_eq!(detail.revisions.len(), 1);
    let revision = &detail.revisions[0];
    assert_eq!(revision.state, RawState::Ready);
    let text = revision.raw_text.as_ref().unwrap();
    assert!(text.len() > 65_536);
    assert_eq!(
        serde_json::from_str::<Value>(text).unwrap()["messages"],
        h["messages"]
    );
    assert!(!text.contains(media.to_str().unwrap()));
    assert_eq!(detail.assets.len(), 1);
    let asset = &detail.assets[0];
    assert_eq!(
        std::fs::read(temp.path().join(&asset.logical_path)).unwrap(),
        b"fixture media bytes"
    );
    let absent = run(&temp, &["collector", "recollect", "--item", item], true);
    assert_eq!(absent["state"], "inaccessible");
    let same = run(
        &temp,
        &[
            "collector",
            "recollect",
            "--item",
            item,
            "--acquisition-handoff",
            first.to_str().unwrap(),
        ],
        true,
    );
    assert_eq!(same["state"], "unchanged");
    assert!(same["new_revision_id"].is_null());
    h["messages"][0]["speechesV2"][0]["content"][0]["msg"] = "updated native content".into();
    std::fs::write(&first, h.to_string()).unwrap();
    let changed = run(
        &temp,
        &[
            "collector",
            "recollect",
            "--item",
            item,
            "--acquisition-handoff",
            first.to_str().unwrap(),
        ],
        true,
    );
    assert_eq!(changed["state"], "changed");
    assert!(changed["new_revision_id"].is_string());
    std::fs::write(&media, b"corrupt media bytes").unwrap();
    let error = run(
        &temp,
        &[
            "collector",
            "recollect",
            "--item",
            item,
            "--acquisition-handoff",
            first.to_str().unwrap(),
        ],
        false,
    );
    assert!(
        error["message"]
            .as_str()
            .unwrap()
            .contains("media size/hash mismatch")
    );
    let after = repository
        .load_detail(&ItemId::parse(item).unwrap())
        .unwrap();
    assert_eq!(after.revisions.len(), 2);
    assert_eq!(after.revisions[0].raw_text, detail.revisions[0].raw_text);
}
