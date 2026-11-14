use std::{
    fs,
    path::Path,
    process::{Child, Command, ExitStatus, Stdio},
    thread,
    time::{Duration, Instant},
};

use babata_application::ApplicationError;
use babata_domain::Sha256;
use serde_json::Value;

pub(crate) fn configured() -> bool {
    let Some(script) =
        std::env::var_os("BABATA_PNG_RENDERER").filter(|value| Path::new(value).is_file())
    else {
        return false;
    };
    let mut command = renderer_command();
    let Ok(mut child) = command
        .arg(script)
        .arg("probe")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };
    wait_for(&mut child, Duration::from_secs(5)).is_ok_and(|status| status.success())
}

fn renderer_command() -> Command {
    let mut command =
        Command::new(std::env::var_os("BABATA_NODE").unwrap_or_else(|| "node".into()));
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    command
}

fn stop_tree(child: &mut Child) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = Command::new("taskkill")
            .args(["/PID", &child.id().to_string(), "/T", "/F"])
            .creation_flags(0x0800_0000)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    #[cfg(unix)]
    {
        let _ = Command::new("kill")
            .args(["-KILL", "--", &format!("-{}", child.id())])
            .status();
    }
    let _ = child.kill();
    let _ = child.wait();
}

fn wait_for(child: &mut Child, timeout: Duration) -> Result<ExitStatus, ApplicationError> {
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return Ok(status),
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(30)),
            Ok(None) => {
                stop_tree(child);
                return Err(ApplicationError::Storage(
                    "PNG renderer timed out; process-tree cleanup requested".to_owned(),
                ));
            }
            Err(error) => {
                stop_tree(child);
                return Err(storage(error));
            }
        }
    }
}

pub(super) fn render(root: &Path, markdown: &str) -> Result<(Vec<u8>, Value), ApplicationError> {
    let script = std::env::var_os("BABATA_PNG_RENDERER").ok_or_else(|| {
        ApplicationError::capability_unavailable("outputs.png", "configure BABATA_PNG_RENDERER")
    })?;
    let runtime = root.join("04_runtime");
    fs::create_dir_all(&runtime).map_err(storage)?;
    let workspace = tempfile::Builder::new()
        .prefix("png-")
        .tempdir_in(runtime)
        .map_err(storage)?;
    let input = workspace.path().join("input.md");
    let output = workspace.path().join("render");
    fs::write(&input, markdown).map_err(storage)?;
    let errors = workspace.path().join("renderer-errors.txt");
    let mut child = renderer_command()
        .arg(script)
        .args(["render", "--input"])
        .arg(&input)
        .arg("--out")
        .arg(&output)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(fs::File::create(&errors).map_err(storage)?)
        .spawn()
        .map_err(storage)?;
    let status = wait_for(&mut child, Duration::from_secs(45))?;
    if !status.success() {
        let detail = fs::read_to_string(errors).unwrap_or_default();
        return Err(ApplicationError::Storage(format!(
            "PNG renderer failed: {}",
            detail.chars().take(2000).collect::<String>()
        )));
    }
    let mut receipt: Value =
        serde_json::from_slice(&fs::read(output.join("manifest.json")).map_err(storage)?)
            .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
    let bytes = fs::read(output.join("output.png")).map_err(storage)?;
    validate(&receipt, markdown, &bytes)?;
    if let Some(object) = receipt.as_object_mut() {
        object.remove("source_input");
        object.remove("source_assets");
        object.remove("files");
    }
    Ok((bytes, receipt))
}

pub(super) fn validate(
    receipt: &Value,
    markdown: &str,
    bytes: &[u8],
) -> Result<(), ApplicationError> {
    let input_hash = Sha256::of_bytes(markdown.as_bytes()).to_string();
    let output_hash = Sha256::of_bytes(bytes).to_string();
    let dimensions = dimensions(bytes)?;
    if receipt["schema_version"] != "babata.png-render/v1"
        || receipt["state"] != "succeeded"
        || receipt["request"]["input_sha256"] != input_hash
        || receipt["request"]["mode"] != "long"
        || receipt["request"]["profile"] != "babata-reading/v1"
        || receipt["outputs"].as_array().map(Vec::len) != Some(1)
        || receipt["outputs"][0]["path"] != "output.png"
        || receipt["outputs"][0]["sha256"] != output_hash
        || receipt["outputs"][0]["width"].as_u64() != Some(u64::from(dimensions.0))
        || receipt["outputs"][0]["height"].as_u64() != Some(u64::from(dimensions.1))
        || receipt["checks"]["assets"] != "pass"
        || receipt["checks"]["structure"] != "pass"
        || receipt["checks"]["dimensions"] != "pass"
        || receipt["checks"]["network"] != "blocked"
        || receipt["checks"]["scripts"] != "disabled"
        || receipt["browser"].as_str().is_none_or(str::is_empty)
    {
        return Err(ApplicationError::Integrity(
            "PNG receipt differs from frozen input or PNG artifact".to_owned(),
        ));
    }
    Ok(())
}

fn dimensions(bytes: &[u8]) -> Result<(u32, u32), ApplicationError> {
    if bytes.len() < 33
        || bytes[..8] != [137, 80, 78, 71, 13, 10, 26, 10]
        || &bytes[12..16] != b"IHDR"
    {
        return Err(ApplicationError::Integrity("invalid PNG header".to_owned()));
    }
    let width = u32::from_be_bytes(bytes[16..20].try_into().expect("fixed PNG width"));
    let height = u32::from_be_bytes(bytes[20..24].try_into().expect("fixed PNG height"));
    if width != 1120 || !(1..=30_000).contains(&height) {
        return Err(ApplicationError::Integrity(
            "invalid PNG dimensions".to_owned(),
        ));
    }
    Ok((width, height))
}

fn storage(error: std::io::Error) -> ApplicationError {
    ApplicationError::Storage(format!("PNG renderer I/O failure: {:?}", error.kind()))
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn deadline_stops_renderer_and_its_descendant() {
        if std::env::var_os("BABATA_PNG_RENDERER").is_none() {
            eprintln!("PNG process-tree test not run: configure renderer test environment");
            return;
        }
        let temporary = tempfile::tempdir().unwrap();
        let pid_file = temporary.path().join("descendant.pid");
        let script = temporary.path().join("timeout.cjs");
        fs::write(&script, "const c=require('node:child_process').spawn(process.execPath,['-e','setInterval(()=>{},1000)'],{stdio:'ignore'});require('node:fs').writeFileSync(process.argv[2],String(c.pid));setInterval(()=>{},1000);").unwrap();
        let mut child = renderer_command()
            .arg(script)
            .arg(&pid_file)
            .spawn()
            .unwrap();
        let startup = Instant::now() + Duration::from_secs(5);
        while !pid_file.exists() && Instant::now() < startup {
            thread::sleep(Duration::from_millis(20));
        }
        if !pid_file.exists() {
            stop_tree(&mut child);
            panic!("descendant did not start");
        }
        let descendant = fs::read_to_string(pid_file).unwrap();
        assert!(wait_for(&mut child, Duration::from_millis(50)).is_err());
        assert!(child.try_wait().unwrap().is_some());
        let listing = Command::new("tasklist")
            .args(["/FI", &format!("PID eq {descendant}"), "/FO", "CSV", "/NH"])
            .output()
            .unwrap();
        assert!(listing.status.success());
        assert!(!String::from_utf8_lossy(&listing.stdout).contains(&format!("\"{descendant}\"")));
    }
}
