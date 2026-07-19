use std::{
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

use babata_application::{
    ApplicationError,
    ports::{ProviderExecutionOutcome, ProviderExecutionRequest, ProviderIdentity},
};
use babata_domain::{CapabilityDescriptor, DerivativeKind, Metadata, ProviderTaskRef};

const MAX_SUMMARY_INPUT_CHARS: usize = 12_000;

#[derive(Debug, Clone)]
pub struct BailianCliConfig {
    pub enabled: bool,
    pub executable: Option<PathBuf>,
    pub version: Option<String>,
}

impl Default for BailianCliConfig {
    fn default() -> Self {
        Self::detect()
    }
}

impl BailianCliConfig {
    pub fn detect() -> Self {
        let candidates = executable_candidates();
        for executable in candidates {
            let Ok(version_output) = Command::new(&executable).arg("--version").output() else {
                continue;
            };
            if !version_output.status.success() {
                continue;
            }
            let version_text = String::from_utf8_lossy(&version_output.stdout);
            let version = version_text
                .trim()
                .strip_prefix("bl ")
                .unwrap_or_else(|| version_text.trim())
                .to_owned();
            let authenticated = Command::new(&executable)
                .args(["auth", "status"])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .ok()
                .is_some_and(|status| status.success());
            return Self {
                enabled: authenticated,
                executable: Some(executable),
                version: Some(version),
            };
        }
        Self {
            enabled: false,
            executable: None,
            version: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct BailianCliProvider {
    config: BailianCliConfig,
}

impl BailianCliProvider {
    pub fn new(config: BailianCliConfig) -> Self {
        Self { config }
    }

    pub fn describe(&self) -> CapabilityDescriptor {
        if self.config.enabled {
            CapabilityDescriptor::enabled("processing.bailian_cli", "P5")
        } else {
            CapabilityDescriptor::unavailable("processing.bailian_cli", "P5")
        }
    }

    pub fn identity(&self) -> ProviderIdentity {
        ProviderIdentity {
            kind: DerivativeKind::Summary,
            provider: "bailian_cli".to_owned(),
            tool_or_model: "qwen-plus".to_owned(),
            tool_version: self
                .config
                .version
                .clone()
                .unwrap_or_else(|| "unknown".to_owned()),
        }
    }

    pub fn execute(
        &self,
        request: &ProviderExecutionRequest,
    ) -> Result<ProviderExecutionOutcome, ApplicationError> {
        if request.pipeline_id.as_str() != "bailian_summary" {
            return Err(ApplicationError::capability_unavailable(
                request.pipeline_id.as_str(),
                "P5",
            ));
        }
        if !self.config.enabled {
            return Err(ApplicationError::capability_unavailable(
                "processing.bailian_cli",
                "P5",
            ));
        }
        let char_count = request.input_text.chars().count();
        if char_count == 0 || char_count > MAX_SUMMARY_INPUT_CHARS {
            return Err(ApplicationError::Integrity(format!(
                "bailian_summary input must contain 1..={MAX_SUMMARY_INPUT_CHARS} characters; got {char_count}"
            )));
        }
        let executable = self.config.executable.as_ref().ok_or_else(|| {
            ApplicationError::capability_unavailable("processing.bailian_cli", "P5")
        })?;
        let response = invoke_chat(executable, &request.input_text)?;
        build_outcome(self.identity(), request, response, char_count)
    }

    pub fn cancel(&self, task: &ProviderTaskRef) -> Result<(), ApplicationError> {
        if task.provider != "bailian_cli" {
            return Err(ApplicationError::Integrity(
                "provider task does not belong to Bailian CLI".to_owned(),
            ));
        }
        // `bl text chat` is synchronous; cancellation is checked before C1
        // commit and prevents its completed response from becoming active C1.
        Ok(())
    }
}

fn invoke_chat(
    executable: &PathBuf,
    input_text: &str,
) -> Result<serde_json::Value, ApplicationError> {
    let messages = serde_json::json!([
        {
            "role": "system",
            "content": "Reply in Simplified Chinese. Produce a faithful concise Markdown summary. Do not add facts absent from the source. State uncertainty when the source is incomplete."
        },
        {
            "role": "user",
            "content": input_text
        }
    ]);
    let mut child = Command::new(executable)
        .args([
            "text",
            "chat",
            "--model",
            "qwen-plus",
            "--messages-file",
            "-",
            "--max-tokens",
            "512",
            "--temperature",
            "0.2",
            "--output",
            "json",
        ])
        .env("NO_COLOR", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            ApplicationError::Provider(format!("Bailian CLI could not start: {error}"))
        })?;
    child
        .stdin
        .take()
        .ok_or_else(|| ApplicationError::Provider("Bailian CLI stdin is unavailable".to_owned()))?
        .write_all(messages.to_string().as_bytes())
        .map_err(|error| {
            ApplicationError::Provider(format!("Bailian CLI stdin failed: {error}"))
        })?;
    let output = child
        .wait_with_output()
        .map_err(|error| ApplicationError::Provider(format!("Bailian CLI wait failed: {error}")))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let detail = safe_error_detail(&stderr);
        return Err(ApplicationError::Provider(format!(
            "Bailian CLI failed with status {}: {}",
            output.status, detail
        )));
    }
    serde_json::from_slice(&output.stdout).map_err(|error| {
        ApplicationError::Provider(format!("Bailian CLI returned invalid JSON: {error}"))
    })
}

fn build_outcome(
    identity: ProviderIdentity,
    request: &ProviderExecutionRequest,
    response: serde_json::Value,
    char_count: usize,
) -> Result<ProviderExecutionOutcome, ApplicationError> {
    let content = extract_content(&response).ok_or_else(|| {
        ApplicationError::Provider("Bailian CLI JSON has no assistant content".to_owned())
    })?;
    ensure_sanitized_content(&content)?;
    let task_id = ["/request_id", "/id", "/task_id", "/output/request_id"]
        .iter()
        .find_map(|pointer| {
            response
                .pointer(pointer)
                .and_then(serde_json::Value::as_str)
        })
        .map_or_else(|| format!("sync:{}", request.job_id), str::to_owned);
    let usage = ["/usage", "/output/usage"]
        .iter()
        .find_map(|pointer| response.pointer(pointer))
        .filter(|value| value.is_object())
        .map(serde_json::Value::to_string)
        .map(|value| Metadata::parse(&value))
        .transpose()
        .map_err(ApplicationError::from)?
        .unwrap_or_else(Metadata::empty);
    let params = Metadata::parse(
        &serde_json::json!({
            "model": "qwen-plus",
            "input_characters": char_count,
            "prompt_version": "summary-v1",
            "remote_provider": true
        })
        .to_string(),
    )
    .map_err(ApplicationError::from)?;
    Ok(ProviderExecutionOutcome {
        task: ProviderTaskRef {
            provider: "bailian_cli".to_owned(),
            task_id,
        },
        kind: identity.kind,
        provider: identity.provider,
        tool_or_model: identity.tool_or_model,
        tool_version: identity.tool_version,
        content_text: Some(content),
        content_json: None,
        media_type: Some("text/markdown".to_owned()),
        language: Some("zh".to_owned()),
        params,
        usage,
        loss_notes: Some(
            "Text summary does not preserve source layout, visual detail, tone, or omitted attachments."
                .to_owned(),
        ),
    })
}

fn executable_candidates() -> Vec<PathBuf> {
    if let Some(configured) = std::env::var_os("BABATA_BAILIAN_CLI") {
        return vec![PathBuf::from(configured)];
    }
    let mut candidates = vec![PathBuf::from("bl"), PathBuf::from("bl.cmd")];
    if let Some(appdata) = std::env::var_os("APPDATA") {
        candidates.push(PathBuf::from(appdata).join("npm/bl.cmd"));
    }
    candidates
}

fn extract_content(value: &serde_json::Value) -> Option<String> {
    [
        "/choices/0/message/content",
        "/output/choices/0/message/content",
        "/output/text",
        "/response/text",
        "/text",
        "/content",
    ]
    .iter()
    .find_map(|pointer| value.pointer(pointer).and_then(serde_json::Value::as_str))
    .map(str::trim)
    .filter(|content| !content.is_empty())
    .map(str::to_owned)
}

fn ensure_sanitized_content(content: &str) -> Result<(), ApplicationError> {
    if contains_sensitive_marker(content) {
        return Err(ApplicationError::Provider(
            "Bailian CLI response contained credential or signed-URL material and was rejected"
                .to_owned(),
        ));
    }
    Ok(())
}

fn safe_error_detail(stderr: &str) -> String {
    if contains_sensitive_marker(stderr) {
        return "provider error detail redacted".to_owned();
    }
    let detail: String = stderr.trim().chars().take(500).collect();
    if detail.is_empty() {
        "no provider error detail".to_owned()
    } else {
        detail
    }
}

fn contains_sensitive_marker(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    [
        "authorization:",
        "cookie:",
        "access_token=",
        "security-token=",
        "x-oss-signature",
        "x-oss-credential",
        "signature=",
        "credential=",
    ]
    .iter()
    .any(|marker| lower.contains(marker))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_openai_compatible_content() {
        let value = serde_json::json!({
            "choices": [{"message": {"content": "summary"}}]
        });
        assert_eq!(extract_content(&value).as_deref(), Some("summary"));
    }

    #[test]
    fn signed_url_material_is_rejected_and_error_details_are_redacted() {
        let signed = "https://example.test/file?x-oss-signature=secret";
        assert!(matches!(
            ensure_sanitized_content(signed),
            Err(ApplicationError::Provider(_))
        ));
        assert_eq!(safe_error_detail(signed), "provider error detail redacted");
    }
}
