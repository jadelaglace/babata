use assert_cmd::Command;
use babata_infrastructure::test_support;
use serde_json::{Value, json};
use tempfile::{TempDir, tempdir};

const ID: &str = "12345678901234567";
fn command(root: &TempDir) -> Command {
    let mut command = Command::cargo_bin("babata").unwrap();
    command.env("BABATA_DATA_HOME", root.path());
    command
}
fn run(root: &TempDir, args: &[&str], success: bool) -> Value {
    let assertion = command(root).args(args).assert();
    let output = if success {
        assertion.success()
    } else {
        assertion.failure()
    };
    serde_json::from_slice(&output.get_output().stdout).unwrap()
}
fn handoff(text: &str) -> Value {
    json!({"protocol_version":"1","provider":"doubao","acquisition_route":"chrome_native",
        "conversation_id":ID,"source_url":format!("https://www.doubao.com/chat/{ID}"),
        "conversation_info":{"conversation_id":ID,"name":"text sample"},
        "messages":[{"conversation_id":ID,"message_id":"message-1","index_in_conv":"1","content_type":9999,"content":text,
            "content_block":[{"block_type":10000,"content":{"text_block":{"text":text}}}]}],
        "has_more":false,"message_cursor":"1","assets":[]})
}
fn start(root: &TempDir, payload: &Value) -> String {
    let path = root.path().join("handoff.json");
    std::fs::write(&path, payload.to_string()).unwrap();
    run(
        root,
        &[
            "--json",
            "collector",
            "start",
            "--route",
            "source.doubao",
            "--source",
            &format!("conversation:{ID}"),
            "--scope",
            "one frozen text",
            "--authorisation",
            "test-auth",
            "--acquisition-handoff",
            path.to_str().unwrap(),
        ],
        true,
    )["session_id"]
        .as_str()
        .unwrap()
        .to_owned()
}
fn args<'a>(session: &'a str, key: &'a str, scope: &'a str) -> Vec<&'a str> {
    vec![
        "--json",
        "collector",
        "commit",
        "--request-key",
        key,
        "--session",
        session,
        "--candidate",
        "doubao_12345678901234567",
        "--scope",
        scope,
        "--authorisation",
        "test-auth",
        "--confirm",
    ]
}
fn counts(root: &TempDir) -> Value {
    // Test-only database access is deliberately separate from product writers.
    let connection =
        test_support::open_raw_connection(&root.path().join("01_raw/index/raw.sqlite"));
    json!([
        connection
            .query_row("SELECT COUNT(*) FROM revisions", [], |row| row
                .get::<_, i64>(0))
            .unwrap(),
        connection
            .query_row("SELECT COUNT(*) FROM source_observations", [], |row| row
                .get::<_, i64>(0))
            .unwrap()
    ])
}
#[test]
fn replay_conflict_new_observation_and_real_change() {
    let root = tempdir().unwrap();
    let session = start(&root, &handoff("original"));
    let first = run(&root, &args(&session, "request-1", "one"), true);
    assert_eq!(first["result"]["state"], "saved");
    assert_eq!(first["protocol_version"], "babata.collection.commit/v1");
    assert_eq!(run(&root, &args(&session, "request-1", "one"), true), first);
    assert_eq!(
        run(&root, &args(&session, "request-1", "changed scope"), false)["code"],
        "conflict"
    );
    assert_eq!(
        run(&root, &args(&session, "request-2", "one"), false)["code"],
        "conflict"
    );
    assert_eq!(counts(&root), json!([1, 1]));
    let second_session = start(&root, &handoff("original"));
    let second = run(&root, &args(&second_session, "observation-2", "one"), true);
    assert_eq!(
        second["result"]["revision_id"],
        first["result"]["revision_id"]
    );
    assert_eq!(counts(&root), json!([1, 2]));
    let changed_session = start(&root, &handoff("changed"));
    let changed = run(&root, &args(&changed_session, "observation-3", "one"), true);
    assert_ne!(
        changed["result"]["revision_id"],
        first["result"]["revision_id"]
    );
    assert_eq!(counts(&root), json!([2, 3]));
    let path = root.path().join("handoff.json");
    run(
        &root,
        &[
            "--json",
            "collector",
            "recollect",
            "--item",
            first["result"]["item_id"].as_str().unwrap(),
            "--acquisition-handoff",
            path.to_str().unwrap(),
        ],
        true,
    );
    assert_eq!(counts(&root), json!([2, 4]));
    assert_eq!(run(&root, &args(&session, "request-1", "one"), true), first);
    assert_eq!(
        run(
            &root,
            &[
                "--json",
                "collector",
                "operation",
                "--request-key",
                "request-1"
            ],
            true
        ),
        first
    );
}
#[test]
fn response_loss_and_ready_before_terminal_reconcile_without_new_writes() {
    let root = tempdir().unwrap();
    let session = start(&root, &handoff("original"));
    let first = run(&root, &args(&session, "crash-1", "one"), true);
    let connection =
        test_support::open_raw_connection(&root.path().join("01_raw/index/raw.sqlite"));
    // Simulate persisted C0 plus process death before collection and request terminal writes.
    connection.execute("UPDATE collection_commit_operations SET operation_json=json_set(operation_json,'$.result',NULL)",[]).unwrap();
    connection
        .execute(
            "UPDATE collection_items SET state='running',item_id=NULL,revision_id=NULL",
            [],
        )
        .unwrap();
    connection
        .execute("UPDATE collection_sessions SET state='running'", [])
        .unwrap();
    drop(connection);
    let recovered = run(&root, &args(&session, "crash-1", "one"), true);
    assert_eq!(
        recovered["result"]["revision_id"],
        first["result"]["revision_id"]
    );
    assert_eq!(recovered["result"]["state"], "saved");
    assert_eq!(counts(&root), json!([1, 1]));
    assert_eq!(
        run(&root, &args(&session, "crash-1", "one"), true),
        recovered
    );
}
#[test]
fn before_capture_interruption_resumes_frozen_input_and_cancel_preserves_saved() {
    let root = tempdir().unwrap();
    let session = start(&root, &handoff("original"));
    let connection =
        test_support::open_raw_connection(&root.path().join("01_raw/index/raw.sqlite"));
    connection.execute("INSERT INTO collection_items (session_id,candidate_id,state,attempt_count,retryable,requested_attachments,created_at,updated_at) VALUES (?1,'doubao_12345678901234567','running',1,0,0,'2026-11-14T00:00:00Z','2026-11-14T00:00:00Z')",[&session]).unwrap();
    connection
        .execute("UPDATE collection_sessions SET state='running'", [])
        .unwrap();
    drop(connection);
    let first = run(&root, &args(&session, "resume-before-capture", "one"), true);
    run(
        &root,
        &[
            "--json",
            "collector",
            "cancel",
            "--session",
            &session,
            "--reason",
            "stop",
        ],
        true,
    );
    assert_eq!(
        run(&root, &args(&session, "resume-before-capture", "one"), true),
        first
    );
    assert_eq!(counts(&root), json!([1, 1]));
    let cancelled = start(&root, &handoff("another"));
    run(
        &root,
        &[
            "--json",
            "collector",
            "cancel",
            "--session",
            &cancelled,
            "--reason",
            "stop",
        ],
        true,
    );
    assert_eq!(
        run(&root, &args(&cancelled, "cancelled", "one"), false)["code"],
        "conflict"
    );
    assert_eq!(counts(&root), json!([1, 1]));
}

#[test]
fn cancellation_is_cooperative_even_while_a_writer_owns_the_lock() {
    use babata_application::ports::CollectionRepositoryPort;
    let root = tempdir().unwrap();
    let session = start(&root, &handoff("original"));
    let paths = babata_infrastructure::paths::DataPaths::new(root.path().to_path_buf());
    let repository = babata_infrastructure::open_collection_database(&paths, 5_000).unwrap();
    let guard = repository.acquire_collection_write().unwrap();
    run(
        &root,
        &[
            "--json",
            "collector",
            "cancel",
            "--session",
            &session,
            "--reason",
            "stop",
        ],
        true,
    );
    repository
        .update_session_state(
            &babata_domain::CollectionSessionId::parse(session.clone()).unwrap(),
            babata_domain::CollectionSessionState::Running,
            &babata_domain::UtcTimestamp::parse("2026-11-14T00:00:00Z").unwrap(),
        )
        .unwrap();
    assert_eq!(
        repository
            .session(&babata_domain::CollectionSessionId::parse(session.clone()).unwrap())
            .unwrap()
            .unwrap()
            .state,
        babata_domain::CollectionSessionState::Cancelled
    );
    drop(guard);
    run(&root, &args(&session, "cancel-race", "one"), false);
    assert_eq!(counts(&root), json!([0, 0]));
}
#[test]
fn corrupted_and_nonready_commits_never_replay_success() {
    for state in ["corrupt", "pending"] {
        let root = tempdir().unwrap();
        let session = start(&root, &handoff("original"));
        run(&root, &args(&session, "integrity", "one"), true);
        let connection =
            test_support::open_raw_connection(&root.path().join("01_raw/index/raw.sqlite"));
        if state == "corrupt" {
            connection
                .execute("UPDATE revisions SET raw_text='corrupted'", [])
                .unwrap();
        } else {
            connection
                .execute("UPDATE revisions SET state='pending'", [])
                .unwrap();
        }
        drop(connection);
        assert_eq!(
            run(&root, &args(&session, "integrity", "one"), false)["code"],
            "integrity_failed"
        );
        assert_eq!(counts(&root), json!([1, 1]));
    }
}
#[test]
fn media_and_incomplete_acquisitions_are_rejected() {
    let root = tempdir().unwrap();
    let mut image = handoff("image");
    image["messages"][0]["content_block"][0]["block_type"] = json!(10052);
    let session = start(&root, &image);
    assert_eq!(
        run(&root, &args(&session, "media", "one"), false)["code"],
        "conflict"
    );
    assert_eq!(counts(&root), json!([0, 0]));
    let mut incomplete = handoff("partial");
    incomplete["has_more"] = json!(true);
    let path = root.path().join("incomplete.json");
    std::fs::write(&path, incomplete.to_string()).unwrap();
    assert_eq!(
        run(
            &root,
            &[
                "--json",
                "collector",
                "start",
                "--route",
                "source.doubao",
                "--source",
                &format!("conversation:{ID}"),
                "--scope",
                "one",
                "--authorisation",
                "test-auth",
                "--acquisition-handoff",
                path.to_str().unwrap()
            ],
            false
        )["code"],
        "integrity_failed"
    );
}
#[test]
fn concurrent_processes_share_one_effect_and_retry_busy_key() {
    let root = tempdir().unwrap();
    let session = start(&root, &handoff("original"));
    let exe = assert_cmd::cargo::cargo_bin!("babata");
    let children = (0..4)
        .map(|_| {
            std::process::Command::new(exe)
                .env("BABATA_DATA_HOME", root.path())
                .args(args(&session, "concurrent", "one"))
                .stdout(std::process::Stdio::piped())
                .spawn()
                .unwrap()
        })
        .collect::<Vec<_>>();
    let mut successes = 0;
    for child in children {
        let output = child.wait_with_output().unwrap();
        let result: Value = serde_json::from_slice(&output.stdout).unwrap();
        if output.status.success() {
            assert_eq!(result["result"]["state"], "saved");
            successes += 1;
        } else {
            assert_eq!(result["code"], "conflict");
            assert!(
                result["message"]
                    .as_str()
                    .unwrap()
                    .contains("collector_busy")
            );
        }
    }
    assert!(successes >= 1);
    run(&root, &args(&session, "concurrent", "one"), true);
    assert_eq!(counts(&root), json!([1, 1]));
}

#[test]
fn frozen_input_drift_cannot_reuse_a_request_key() {
    let root = tempdir().unwrap();
    let session = start(&root, &handoff("original"));
    run(&root, &args(&session, "bound-input", "one"), true);
    let connection =
        test_support::open_raw_connection(&root.path().join("01_raw/index/raw.sqlite"));
    connection
        .execute(
            "UPDATE collection_candidates SET title='different frozen summary'",
            [],
        )
        .unwrap();
    drop(connection);
    assert_eq!(
        run(&root, &args(&session, "bound-input", "one"), false)["code"],
        "conflict"
    );
    assert_eq!(counts(&root), json!([1, 1]));
}

#[test]
fn nested_media_missing_indices_and_live_only_candidates_are_rejected() {
    for kind in ["nested-media", "missing-index"] {
        let root = tempdir().unwrap();
        let mut value = handoff("text");
        if kind == "nested-media" {
            value["messages"][0]["tts_content"] = json!(json!({"entities":[{"entity_content":{"image":{"image_ori":{"url":"https://example.test/image"}}}}]}).to_string());
        } else {
            value["messages"][0]["index_in_conv"] = json!(2);
        }
        let session = start(&root, &value);
        run(&root, &args(&session, kind, "one"), false);
        assert_eq!(counts(&root), json!([0, 0]));
    }
    let root = tempdir().unwrap();
    let session = run(
        &root,
        &[
            "--json",
            "collector",
            "start",
            "--route",
            "source.doubao",
            "--source",
            &format!("conversation:{ID}"),
            "--scope",
            "one",
            "--authorisation",
            "test-auth",
        ],
        true,
    )["session_id"]
        .as_str()
        .unwrap()
        .to_owned();
    assert_eq!(
        run(&root, &args(&session, "live", "one"), false)["code"],
        "conflict"
    );
}

#[test]
fn cross_session_writers_and_legacy_select_share_the_lock() {
    use babata_application::ports::CollectionRepositoryPort;
    let root = tempdir().unwrap();
    let session = start(&root, &handoff("original"));
    let paths = babata_infrastructure::paths::DataPaths::new(root.path().to_path_buf());
    let repository = babata_infrastructure::open_collection_database(&paths, 5_000).unwrap();
    let guard = repository.acquire_collection_write().unwrap();
    assert_eq!(
        run(&root, &args(&session, "locked", "one"), false)["code"],
        "conflict"
    );
    assert_eq!(
        run(
            &root,
            &[
                "--json",
                "collector",
                "select",
                "--session",
                &session,
                "--candidate",
                "doubao_12345678901234567",
                "--scope",
                "one",
                "--authorisation",
                "test-auth",
                "--confirm"
            ],
            false
        )["code"],
        "conflict"
    );
    assert_eq!(counts(&root), json!([0, 0]));
    drop(guard);
    run(&root, &args(&session, "locked", "one"), true);
    assert_eq!(counts(&root), json!([1, 1]));
}
