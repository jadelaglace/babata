use std::{
    io::Write,
    process::{Command, Stdio},
};

use babata_application::{
    ApplicationError,
    ports::{SemanticDigestOutcome, SemanticDigestProviderPort, SemanticDigestRequest},
};
use babata_domain::{
    CapabilityDescriptor, Metadata, SEMANTIC_CANDIDATE_SCHEMA_V1, SemanticCandidateBody,
    SemanticCandidatePackage,
};

use super::bailian_cli::BailianCliConfig;

const MAX_REVIEW_CONTEXT_CHARS: usize = 30_000;
const PROMPT_VERSION: &str = "p6-semantic-v1";

#[derive(Debug, Clone, Default)]
pub struct BailianSemanticDigestProvider {
    config: BailianCliConfig,
}

impl BailianSemanticDigestProvider {
    pub fn detect() -> Self {
        Self {
            config: BailianCliConfig::detect(),
        }
    }

    pub fn describe(&self) -> CapabilityDescriptor {
        if self.config.enabled {
            CapabilityDescriptor::enabled("knowledge.semantic_digest.bailian_cli", "P6.1")
        } else {
            CapabilityDescriptor::unavailable("knowledge.semantic_digest.bailian_cli", "P6.1")
        }
    }
}

impl SemanticDigestProviderPort for BailianSemanticDigestProvider {
    #[allow(clippy::too_many_lines)]
    fn execute(
        &self,
        request: &SemanticDigestRequest,
    ) -> Result<SemanticDigestOutcome, ApplicationError> {
        if !self.config.enabled {
            return Err(ApplicationError::capability_unavailable(
                "knowledge.semantic_digest.bailian_cli",
                "P6.1",
            ));
        }
        let character_count = request.review_context.chars().count();
        if character_count == 0 || character_count > MAX_REVIEW_CONTEXT_CHARS {
            return Err(ApplicationError::Integrity(format!(
                "semantic review context must contain 1..={MAX_REVIEW_CONTEXT_CHARS} characters; got {character_count}"
            )));
        }
        let executable = self.config.executable.as_ref().ok_or_else(|| {
            ApplicationError::capability_unavailable(
                "knowledge.semantic_digest.bailian_cli",
                "P6.1",
            )
        })?;
        let messages = serde_json::json!([
            {
                "role": "system",
                "content": system_prompt()
            },
            {
                "role": "user",
                "content": request.review_context
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
                "4096",
                "--temperature",
                "0.1",
                "--output",
                "json",
            ])
            .env("NO_COLOR", "1")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|error| {
                ApplicationError::Provider(format!(
                    "Bailian semantic digest could not start: {error}"
                ))
            })?;
        child
            .stdin
            .take()
            .ok_or_else(|| {
                ApplicationError::Provider(
                    "Bailian semantic digest stdin is unavailable".to_owned(),
                )
            })?
            .write_all(messages.to_string().as_bytes())
            .map_err(|error| {
                ApplicationError::Provider(format!("Bailian semantic digest stdin failed: {error}"))
            })?;
        let output = child.wait_with_output().map_err(|error| {
            ApplicationError::Provider(format!("Bailian semantic digest wait failed: {error}"))
        })?;
        if !output.status.success() {
            let detail = safe_error_detail(&String::from_utf8_lossy(&output.stderr));
            return Err(ApplicationError::Provider(format!(
                "Bailian semantic digest failed with status {}: {detail}",
                output.status
            )));
        }
        let response: serde_json::Value =
            serde_json::from_slice(&output.stdout).map_err(|error| {
                ApplicationError::Provider(format!(
                    "Bailian semantic digest returned invalid JSON: {error}"
                ))
            })?;
        let content = extract_content(&response).ok_or_else(|| {
            ApplicationError::Provider(
                "Bailian semantic digest JSON has no assistant content".to_owned(),
            )
        })?;
        let body: SemanticCandidateBody = serde_json::from_str(strip_json_fence(&content))
            .map_err(|error| {
                ApplicationError::Provider(format!(
                    "Bailian semantic digest did not follow the candidate contract: {error}"
                ))
            })?;
        let package = SemanticCandidatePackage {
            schema_version: SEMANTIC_CANDIDATE_SCHEMA_V1.to_owned(),
            source_item_id: request.source_item_id.clone(),
            source_revision_id: request.source_revision_id.clone(),
            evidence_derivatives: request.evidence.clone(),
            provider: "bailian_cli".to_owned(),
            model: "qwen-plus".to_owned(),
            model_version: self
                .config
                .version
                .clone()
                .unwrap_or_else(|| "unknown".to_owned()),
            prompt_version: PROMPT_VERSION.to_owned(),
            generated_at: request.generated_at.clone(),
            map_nodes: body.map_nodes,
            entries: body.entries,
            relations: body.relations,
            limitations: body.limitations,
        };
        package.validate().map_err(ApplicationError::from)?;
        Ok(SemanticDigestOutcome {
            package,
            provider_task_id: task_id(&response),
            usage: usage(&response)?,
        })
    }
}

fn system_prompt() -> &'static str {
    r#"你是 Babata P6.1 语义消化器。只根据输入中的 C0 原文和 C1 证据生成机器候选，不得把模型判断写成用户观点或确认事实。只输出一个 JSON 对象，不要 Markdown 围栏，不要解释。

JSON 顶层必须只有：map_nodes、entries、relations、limitations。

map_nodes: 动态学科/分支数组。每项为 {"local_ref":"node:...","level":"discipline|branch","name":"...","parent_refs":["foundation:time|foundation:space|foundation:matter|foundation:consciousness|node:..."]}。不得创建 foundation。学科直接连一个或多个 foundation，分支连接学科；按需支持多父级。

entries: 至少一项。每项为 {"local_ref":"entry:...","title":"...","payload":...,"map_node_refs":[...],"tags":[...],"dense_expressions":[...],"relevance":{"interest":0到100整数,"strategy":0到100整数,"consensus":0到100整数,"rationale":"依据"}}。

payload 必须是以下之一：
{"kind":"knowledge","statement":"高浓缩知识判断","details":"忠实详细解释"}
{"kind":"case","scenario":"场景","process":"过程","result":"结果","analysis":"分析反思"}
{"kind":"map_direction","description":"方向描述"}

外部资料不得生成 log 或 insight，那两类属于用户第一方认知轨迹。Case 只在原文有可辨认实践场景、过程和结果时生成。若同时有 Knowledge 和 Case，用 relations 建立 applied_by、validates、challenges 等有方向关系。

dense_expressions 每项为 {"kind":"mind_map|mermaid|model|formula|checklist|process|outline","content":"可编辑文本"}。至少一条 Knowledge 必须有高密度表达；Mermaid 必须是可解析源码而不是图片。

relations 每项为 {"from_ref":"entry:...","kind":"...","to_ref":"entry:...","evidence":"原文依据"}，端点只能引用本包 entries 且不能自指。limitations 是模型不确定性和资料缺口字符串数组。"#
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

fn strip_json_fence(content: &str) -> &str {
    content
        .strip_prefix("```json")
        .or_else(|| content.strip_prefix("```"))
        .and_then(|value| value.strip_suffix("```"))
        .map_or(content, str::trim)
}

fn task_id(value: &serde_json::Value) -> String {
    ["/request_id", "/id", "/task_id", "/output/request_id"]
        .iter()
        .find_map(|pointer| value.pointer(pointer).and_then(serde_json::Value::as_str))
        .map_or_else(|| "synchronous".to_owned(), str::to_owned)
}

fn usage(value: &serde_json::Value) -> Result<Metadata, ApplicationError> {
    ["/usage", "/output/usage"]
        .iter()
        .find_map(|pointer| value.pointer(pointer))
        .filter(|usage| usage.is_object())
        .map(serde_json::Value::to_string)
        .map(|usage| Metadata::parse(&usage).map_err(ApplicationError::from))
        .transpose()
        .map(|usage| usage.unwrap_or_else(Metadata::empty))
}

fn safe_error_detail(stderr: &str) -> String {
    let lower = stderr.to_ascii_lowercase();
    if [
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
    {
        return "provider error detail redacted".to_owned();
    }
    let detail = stderr.trim().chars().take(500).collect::<String>();
    if detail.is_empty() {
        "no provider error detail".to_owned()
    } else {
        detail
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_optional_json_fence() {
        assert_eq!(strip_json_fence("```json\n{}\n```"), "{}");
        assert_eq!(strip_json_fence("{}"), "{}");
    }
}
