use assert_cmd::Command;
use babata_infrastructure::test_support::{raw_authority_digest, revision_mutation_rejections};
use serde_json::{Value, json};
use tempfile::tempdir;

fn babata(temp: &tempfile::TempDir) -> Command {
    let mut command = Command::cargo_bin("babata").unwrap();
    command.env("BABATA_DATA_HOME", temp.path());
    command
}

fn run_json(temp: &tempfile::TempDir, args: &[&str]) -> Value {
    let output = babata(temp)
        .arg("--json")
        .args(args)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&output).unwrap()
}

fn capture(temp: &tempfile::TempDir, text: &str) -> Value {
    run_json(
        temp,
        &["capture", "text", "--provider", "fixture", "--text", text],
    )
}

fn authority_digest(temp: &tempfile::TempDir) -> (i64, i64, String) {
    raw_authority_digest(temp.path()).unwrap()
}

#[test]
#[allow(clippy::too_many_lines)]
fn p6_3_versions_materializes_and_outputs_without_writing_back_to_authority() {
    let temp = tempdir().unwrap();
    let included = capture(&temp, "P6.3 included source evidence");
    let excluded = capture(&temp, "P6.3 excluded source evidence");
    let manually_included = run_json(
        &temp,
        &[
            "create",
            "--text",
            "P6.3 manually included first-party evidence",
        ],
    );
    let included_record = format!("item:{}", included["item_id"].as_str().unwrap());
    let excluded_record = format!("item:{}", excluded["item_id"].as_str().unwrap());
    let manually_included_record =
        format!("item:{}", manually_included["item_id"].as_str().unwrap());
    run_json(&temp, &["explore", "rebuild"]);

    let definition_path = temp.path().join("definition.json");
    std::fs::write(
        &definition_path,
        serde_json::to_vec_pretty(&json!({
            "title": "P6.3 fixture library",
            "purpose": "Organise one explicit fixture scope without a second authority",
            "selection": {"provider": "fixture", "limit": 20},
            "manual_include": [manually_included_record],
            "manual_exclude": [excluded_record],
            "organisation_rules": ["manual_first", "title"],
            "include_unreviewed": false
        }))
        .unwrap(),
    )
    .unwrap();
    let created = run_json(
        &temp,
        &[
            "sublibraries",
            "create",
            "--definition",
            &definition_path.to_string_lossy(),
        ],
    );
    let sublibrary = created["id"].as_str().unwrap();
    let item_id = created["authority"]["item_id"].as_str().unwrap();
    let revision_v1 = created["authority"]["revision_id"].as_str().unwrap();
    assert_eq!(created["version"], 1);

    let revised_path = temp.path().join("definition-v2.json");
    std::fs::write(
        &revised_path,
        serde_json::to_vec_pretty(&json!({
            "title": "P6.3 fixture library revised",
            "purpose": "Preserve a second explicit first-party definition version",
            "selection": {"provider": "fixture", "limit": 20},
            "manual_include": [manually_included_record],
            "manual_exclude": [excluded_record],
            "organisation_rules": ["source_then_title"],
            "include_unreviewed": true
        }))
        .unwrap(),
    )
    .unwrap();
    let revised = run_json(
        &temp,
        &[
            "sublibraries",
            "revise",
            "--sublibrary",
            sublibrary,
            "--expected-version",
            "1",
            "--definition",
            &revised_path.to_string_lossy(),
        ],
    );
    assert_eq!(revised["version"], 2);
    assert_eq!(revised["authority"]["item_id"], item_id);
    assert_ne!(revised["authority"]["revision_id"], revision_v1);
    let versions = run_json(
        &temp,
        &["sublibraries", "versions", "--sublibrary", sublibrary],
    );
    assert_eq!(versions.as_array().unwrap().len(), 2);

    let before_c2 = authority_digest(&temp);
    let materialized = run_json(
        &temp,
        &[
            "sublibraries",
            "materialize",
            "--sublibrary",
            sublibrary,
            "--version",
            "1",
        ],
    );
    assert_eq!(materialized["member_count"], 2);
    assert_eq!(materialized["state"], "succeeded");
    let materialization_path = temp
        .path()
        .join(materialized["materialization_path"].as_str().unwrap());
    let materialization: Value =
        serde_json::from_slice(&std::fs::read(&materialization_path).unwrap()).unwrap();
    assert_eq!(
        materialization["members"][0]["record"]["record_id"],
        manually_included_record
    );
    assert_eq!(
        materialization["members"][0]["inclusion_reasons"],
        json!(["manual_include"])
    );
    assert_eq!(
        materialization["members"][1]["record"]["record_id"],
        included_record
    );
    assert_eq!(
        materialization["exclusions"][0]["record_id"],
        excluded_record
    );
    run_json(
        &temp,
        &[
            "sublibraries",
            "verify",
            "--sublibrary",
            sublibrary,
            "--version",
            "1",
        ],
    );
    let materialization_manifest_path = temp
        .path()
        .join(materialized["manifest_path"].as_str().unwrap());
    let original_materialization_manifest = std::fs::read(&materialization_manifest_path).unwrap();
    let mut changed_materialization_manifest: Value =
        serde_json::from_slice(&original_materialization_manifest).unwrap();
    changed_materialization_manifest["projection_fingerprint"] = json!("f".repeat(64));
    std::fs::write(
        &materialization_manifest_path,
        serde_json::to_vec_pretty(&changed_materialization_manifest).unwrap(),
    )
    .unwrap();
    babata(&temp)
        .args([
            "sublibraries",
            "verify",
            "--sublibrary",
            sublibrary,
            "--version",
            "1",
        ])
        .assert()
        .failure();
    std::fs::write(
        &materialization_manifest_path,
        original_materialization_manifest,
    )
    .unwrap();
    std::fs::write(&materialization_path, b"tampered materialization").unwrap();
    babata(&temp)
        .args([
            "sublibraries",
            "verify",
            "--sublibrary",
            sublibrary,
            "--version",
            "1",
        ])
        .assert()
        .failure();
    run_json(
        &temp,
        &[
            "sublibraries",
            "delete",
            "--sublibrary",
            sublibrary,
            "--version",
            "1",
        ],
    );
    run_json(
        &temp,
        &[
            "sublibraries",
            "rebuild",
            "--sublibrary",
            sublibrary,
            "--version",
            "1",
        ],
    );

    let human = run_json(
        &temp,
        &[
            "outputs",
            "build",
            "--kind",
            "human_readable",
            "--sublibrary",
            sublibrary,
            "--sublibrary-version",
            "1",
            "--description",
            "P6.3 human fixture output",
        ],
    );
    let human_id = human["id"].as_str().unwrap();
    let human_path = temp.path().join(human["artifact_path"].as_str().unwrap());
    let human_text = std::fs::read_to_string(&human_path).unwrap();
    assert!(human_text.contains(&included_record));
    assert!(human_text.contains("human_judgment"));
    assert!(
        run_json(&temp, &["outputs", "verify", "--output", human_id])["valid"]
            .as_bool()
            .unwrap()
    );
    let human_manifest_path = temp
        .path()
        .join("03_views/outputs")
        .join(human_id)
        .join("manifest.json");
    let original_human_manifest = std::fs::read(&human_manifest_path).unwrap();
    let sentinel = temp.path().join("must-not-delete.txt");
    std::fs::write(&sentinel, b"outside the output directory").unwrap();
    let mut escaped_manifest: Value = serde_json::from_slice(&original_human_manifest).unwrap();
    escaped_manifest["artifact_file"] = json!("../../../must-not-delete.txt");
    std::fs::write(
        &human_manifest_path,
        serde_json::to_vec_pretty(&escaped_manifest).unwrap(),
    )
    .unwrap();
    babata(&temp)
        .args(["outputs", "delete", "--output", human_id])
        .assert()
        .failure();
    assert!(sentinel.exists());
    std::fs::write(&human_manifest_path, &original_human_manifest).unwrap();

    let mut changed_hash_manifest: Value =
        serde_json::from_slice(&original_human_manifest).unwrap();
    changed_hash_manifest["output_sha256"] = json!("f".repeat(64));
    std::fs::write(
        &human_manifest_path,
        serde_json::to_vec_pretty(&changed_hash_manifest).unwrap(),
    )
    .unwrap();
    assert_eq!(
        run_json(&temp, &["outputs", "verify", "--output", human_id])["valid"],
        false
    );
    std::fs::write(&human_manifest_path, original_human_manifest).unwrap();

    let original_human_manifest = std::fs::read(&human_manifest_path).unwrap();
    let mut changed_differences_manifest: Value =
        serde_json::from_slice(&original_human_manifest).unwrap();
    changed_differences_manifest["differences"] = json!(["fabricated difference"]);
    std::fs::write(
        &human_manifest_path,
        serde_json::to_vec_pretty(&changed_differences_manifest).unwrap(),
    )
    .unwrap();
    assert_eq!(
        run_json(&temp, &["outputs", "verify", "--output", human_id])["valid"],
        false
    );
    std::fs::write(&human_manifest_path, original_human_manifest).unwrap();

    let structured = run_json(
        &temp,
        &[
            "outputs",
            "build",
            "--kind",
            "structured",
            "--record",
            &included_record,
            "--record",
            &excluded_record,
            "--description",
            "P6.3 explicit record-set fixture output",
        ],
    );
    let structured_path = temp
        .path()
        .join(structured["artifact_path"].as_str().unwrap());
    let structured_json: Value =
        serde_json::from_slice(&std::fs::read(structured_path).unwrap()).unwrap();
    assert_eq!(
        structured_json["schema_version"],
        "babata.structured-output/v1"
    );
    assert_eq!(
        structured_json["records"][0]["detail"]["record"]["record_id"],
        included_record
    );
    assert_eq!(structured_json["records"].as_array().unwrap().len(), 2);

    std::fs::write(&human_path, b"externally edited").unwrap();
    let tampered = run_json(&temp, &["outputs", "verify", "--output", human_id]);
    assert_eq!(tampered["valid"], false);
    assert!(tampered["actual_sha256"].as_str().is_some());
    let deleted = run_json(&temp, &["outputs", "delete", "--output", human_id]);
    assert_eq!(deleted["state"], "deleted");
    assert!(!human_path.exists());
    let rebuilt = run_json(&temp, &["outputs", "rebuild", "--output", human_id]);
    assert_eq!(rebuilt["generation"], 2);
    assert_eq!(rebuilt["state"], "succeeded");
    assert!(human_path.exists());
    assert!(
        run_json(&temp, &["outputs", "verify", "--output", human_id])["valid"]
            .as_bool()
            .unwrap()
    );
    let rebuilt_manifest = std::fs::read(&human_manifest_path).unwrap();
    let mut changed_history_manifest: Value = serde_json::from_slice(&rebuilt_manifest).unwrap();
    changed_history_manifest["previous_manifest_sha256"] = json!("f".repeat(64));
    std::fs::write(
        &human_manifest_path,
        serde_json::to_vec_pretty(&changed_history_manifest).unwrap(),
    )
    .unwrap();
    babata(&temp)
        .args(["outputs", "verify", "--output", human_id])
        .assert()
        .failure();
    std::fs::write(&human_manifest_path, rebuilt_manifest).unwrap();

    babata(&temp)
        .args([
            "outputs",
            "build",
            "--kind",
            "web",
            "--record",
            &included_record,
            "--description",
            "unsupported fixture output",
        ])
        .assert()
        .failure();
    assert_eq!(authority_digest(&temp), before_c2);

    let (update_error, delete_error) =
        revision_mutation_rejections(temp.path(), revision_v1).unwrap();
    assert!(update_error.contains("immutable"));
    assert!(delete_error.contains("append-only"));
}

#[test]
fn output_verify_recomputes_current_authoritative_inputs() {
    let temp = tempdir().unwrap();
    let authored = run_json(&temp, &["create", "--text", "first authoritative version"]);
    let item_record = format!("item:{}", authored["item_id"].as_str().unwrap());
    run_json(&temp, &["explore", "rebuild"]);
    let output = run_json(
        &temp,
        &[
            "outputs",
            "build",
            "--kind",
            "structured",
            "--record",
            &item_record,
            "--description",
            "Current authoritative input verification",
        ],
    );
    let output_id = output["id"].as_str().unwrap();
    assert_eq!(
        run_json(&temp, &["outputs", "verify", "--output", output_id])["valid"],
        true
    );

    run_json(
        &temp,
        &[
            "revise",
            "--parent",
            authored["revision_id"].as_str().unwrap(),
            "--text",
            "second authoritative version",
        ],
    );
    run_json(&temp, &["explore", "rebuild"]);
    assert_eq!(
        run_json(&temp, &["outputs", "verify", "--output", output_id])["valid"],
        false
    );
}
