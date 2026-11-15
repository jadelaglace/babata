use assert_cmd::Command;
use babata_infrastructure::test_support::raw_authority_digest;
use serde_json::{Value, json};
use tempfile::tempdir;

fn run(root: &std::path::Path, args: &[&str]) -> Value {
    let result = Command::cargo_bin("babata")
        .unwrap()
        .env("BABATA_DATA_HOME", root)
        .arg("--json")
        .args(args)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    serde_json::from_slice(&result).unwrap()
}

fn verify(root: &std::path::Path, id: &str, expected: bool) {
    assert_eq!(
        run(root, &["outputs", "verify", "--output", id])["valid"],
        expected
    );
}

fn failed_rebuild_preserves(
    root: &std::path::Path,
    id: &str,
    file: &std::path::Path,
    manifest: &std::path::Path,
) {
    let original = std::fs::read(file).unwrap();
    let manifest_bytes = std::fs::read(manifest).unwrap();
    Command::cargo_bin("babata")
        .unwrap()
        .env("BABATA_DATA_HOME", root)
        .env(
            "BABATA_CHROMIUM_EXECUTABLE",
            root.join("missing-browser.exe"),
        )
        .args(["outputs", "rebuild", "--output", id])
        .assert()
        .failure();
    assert_eq!(std::fs::read(file).unwrap(), original);
    assert_eq!(std::fs::read(manifest).unwrap(), manifest_bytes);
    verify(root, id, true);
}

#[test]
fn real_browser_png_lifecycle_preserves_authority_and_detects_tampering() {
    if std::env::var_os("BABATA_PNG_RENDERER").is_none() {
        eprintln!("real PNG lifecycle not run: set BABATA_PNG_RENDERER and browser dependencies");
        return;
    }
    let temp = tempdir().unwrap();
    let root = temp.path();
    let created = run(
        root,
        &[
            "create",
            "--text",
            "Babata PNG 独立能力：已有知识可以离线输出，不重新调研。",
        ],
    );
    let record = format!("item:{}", created["item_id"].as_str().unwrap());
    run(root, &["explore", "rebuild"]);
    let before = raw_authority_digest(root).unwrap();
    let built = run(
        root,
        &[
            "outputs",
            "build",
            "--kind",
            "png",
            "--record",
            &record,
            "--description",
            "中文正式 PNG 输出验证",
        ],
    );
    assert_eq!(built["kind"], "png");
    assert_eq!(built["state"], "succeeded");
    let id = built["id"].as_str().unwrap();
    let file = root.join(built["artifact_path"].as_str().unwrap());
    let manifest_file = root.join(built["manifest"]["relative_path"].as_str().unwrap());
    let original = std::fs::read(&file).unwrap();
    assert_eq!(&original[..8], &[137, 80, 78, 71, 13, 10, 26, 10]);
    verify(root, id, true);
    assert_eq!(
        run(root, &["outputs", "status", "--output", id])["generation"],
        1
    );
    let manifest_bytes = std::fs::read(&manifest_file).unwrap();
    failed_rebuild_preserves(root, id, &file, &manifest_file);
    let mut changed: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    changed["png_receipt"]["browser"] = json!("tampered-browser");
    std::fs::write(&manifest_file, serde_json::to_vec(&changed).unwrap()).unwrap();
    verify(root, id, false);
    std::fs::write(&manifest_file, &manifest_bytes).unwrap();
    std::fs::write(&file, b"invalid image").unwrap();
    assert_eq!(
        run(root, &["outputs", "verify", "--output", id])["valid"],
        false
    );
    std::fs::write(&file, &original).unwrap();
    run(
        root,
        &[
            "revise",
            "--parent",
            created["revision_id"].as_str().unwrap(),
            "--text",
            "Changed authoritative content.",
        ],
    );
    run(root, &["explore", "rebuild"]);
    assert_eq!(
        run(root, &["outputs", "verify", "--output", id])["valid"],
        false
    );
    let after_revision = raw_authority_digest(root).unwrap();
    assert_ne!(after_revision, before);
    assert_eq!(
        run(root, &["outputs", "rebuild", "--output", id])["generation"],
        2
    );
    assert_eq!(
        run(root, &["outputs", "verify", "--output", id])["valid"],
        true
    );
    assert_eq!(
        run(root, &["outputs", "delete", "--output", id])["state"],
        "deleted"
    );
    assert!(!file.exists());
    assert_eq!(
        run(root, &["outputs", "verify", "--output", id])["valid"],
        false
    );
    assert_eq!(
        run(root, &["outputs", "rebuild", "--output", id])["generation"],
        3
    );
    assert_eq!(
        run(root, &["outputs", "verify", "--output", id])["valid"],
        true
    );
    assert_eq!(raw_authority_digest(root).unwrap(), after_revision);
}

#[test]
fn missing_renderer_rejects_without_successful_output() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    let created = run(
        root,
        &[
            "create",
            "--text",
            "No renderer must not silently produce Markdown.",
        ],
    );
    let record = format!("item:{}", created["item_id"].as_str().unwrap());
    run(root, &["explore", "rebuild"]);
    Command::cargo_bin("babata")
        .unwrap()
        .env("BABATA_DATA_HOME", root)
        .env_remove("BABATA_PNG_RENDERER")
        .args([
            "outputs",
            "build",
            "--kind",
            "png",
            "--record",
            &record,
            "--description",
            "Unavailable renderer",
        ])
        .assert()
        .failure();
    assert!(
        !root.join("03_views/outputs").exists()
            || std::fs::read_dir(root.join("03_views/outputs"))
                .unwrap()
                .next()
                .is_none()
    );
}
