use std::fmt::Write;

use serde::Serialize;

use crate::{
    ApplicationError, DenseExpressionDetail, DenseExpressionPreviewDocument,
    DenseExpressionPreviewOutcome, SemanticEntryDetail,
    ports::{DenseExpressionPreviewPort, KnowledgeCoreRepositoryPort},
};
use babata_domain::{DenseExpressionKind, KnowledgeKind, SemanticPayload, Sha256};

pub struct DenseExpressionPreviewService<R, V> {
    repository: R,
    views: V,
}

impl<R, V> DenseExpressionPreviewService<R, V>
where
    R: KnowledgeCoreRepositoryPort,
    V: DenseExpressionPreviewPort,
{
    pub fn new(repository: R, views: V) -> Self {
        Self { repository, views }
    }

    pub fn build(
        &self,
        semantic_id: &str,
    ) -> Result<DenseExpressionPreviewOutcome, ApplicationError> {
        let entry = self.repository.load_semantic_entry(semantic_id)?;
        let document = preview_document(&entry)?;
        self.views.build(&document)
    }

    pub fn verify(
        &self,
        semantic_id: &str,
    ) -> Result<DenseExpressionPreviewOutcome, ApplicationError> {
        let entry = self.repository.load_semantic_entry(semantic_id)?;
        let document = preview_document(&entry)?;
        self.views.verify(semantic_id, &document.source_sha256)
    }

    pub fn delete(
        &self,
        semantic_id: &str,
    ) -> Result<DenseExpressionPreviewOutcome, ApplicationError> {
        self.repository.load_semantic_entry(semantic_id)?;
        self.views.delete(semantic_id)
    }
}

#[derive(Serialize)]
struct PreviewSource<'a> {
    semantic_id: &'a str,
    title: &'a str,
    kind: KnowledgeKind,
    origin_kind: &'a str,
    author: &'a str,
    payload: &'a SemanticPayload,
    dense_expressions: &'a [DenseExpressionDetail],
}

fn preview_document(
    entry: &SemanticEntryDetail,
) -> Result<DenseExpressionPreviewDocument, ApplicationError> {
    if entry.dense_expressions.is_empty() {
        return Err(ApplicationError::Conflict(format!(
            "semantic entry {} has no high-density text to preview",
            entry.semantic_id
        )));
    }
    let source = PreviewSource {
        semantic_id: &entry.semantic_id,
        title: &entry.title,
        kind: entry.kind,
        origin_kind: &entry.origin_kind,
        author: &entry.author,
        payload: &entry.payload,
        dense_expressions: &entry.dense_expressions,
    };
    let source_json = serde_json::to_vec(&source)
        .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
    let payload = serde_json::to_string_pretty(&entry.payload)
        .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
    let mut markdown = format!(
        "# {}\n\n- Semantic ID: `{}`\n- Kind: `{:?}`\n- Origin: `{}`\n- Author: `{}`\n\n## Structured content\n\n{}\n",
        one_line(&entry.title),
        entry.semantic_id,
        entry.kind,
        entry.origin_kind,
        one_line(&entry.author),
        fenced("json", &payload),
    );
    for expression in &entry.dense_expressions {
        write!(
            markdown,
            "\n## {:?}\n\n{}\n",
            expression.kind,
            fenced(expression_language(expression.kind), &expression.content),
        )
        .map_err(|_| ApplicationError::Integrity("failed to render C2 preview".to_owned()))?;
    }
    Ok(DenseExpressionPreviewDocument {
        semantic_id: entry.semantic_id.clone(),
        source_sha256: Sha256::of_bytes(&source_json),
        markdown,
    })
}

fn one_line(value: &str) -> String {
    value
        .lines()
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

const fn expression_language(kind: DenseExpressionKind) -> &'static str {
    match kind {
        DenseExpressionKind::Mermaid | DenseExpressionKind::MindMap => "mermaid",
        DenseExpressionKind::Formula => "text",
        DenseExpressionKind::Model
        | DenseExpressionKind::Checklist
        | DenseExpressionKind::Process
        | DenseExpressionKind::Outline => "markdown",
    }
}

fn fenced(language: &str, content: &str) -> String {
    let mut longest = 0;
    let mut current = 0;
    for character in content.chars() {
        if character == '`' {
            current += 1;
            longest = longest.max(current);
        } else {
            current = 0;
        }
    }
    let fence = "`".repeat(longest.max(2) + 1);
    format!("{fence}{language}\n{content}\n{fence}")
}

#[cfg(test)]
mod tests {
    use super::{fenced, one_line};

    #[test]
    fn fenced_blocks_expand_around_embedded_backticks() {
        let rendered = fenced("markdown", "before ``` inside");
        assert!(rendered.starts_with("````markdown\n"));
        assert!(rendered.ends_with("\n````"));
    }

    #[test]
    fn preview_headings_escape_inline_html() {
        assert_eq!(one_line("<script>&\nname"), "&lt;script&gt;&amp; name");
    }
}
