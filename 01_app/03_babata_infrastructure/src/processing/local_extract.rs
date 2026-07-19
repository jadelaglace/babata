use babata_application::{
    ApplicationError,
    ports::{ProviderExecutionOutcome, ProviderExecutionRequest, ProviderIdentity},
};
use babata_domain::{CapabilityDescriptor, DerivativeKind, Metadata, ProviderTaskRef, Sha256};

#[derive(Debug, Clone, Default)]
pub struct LocalExtractProvider;

impl LocalExtractProvider {
    pub fn describe(&self) -> CapabilityDescriptor {
        CapabilityDescriptor::enabled("processing.local_extract", "P5")
    }

    pub fn identity(&self) -> ProviderIdentity {
        ProviderIdentity {
            kind: DerivativeKind::ExtractedText,
            provider: "local_extract".to_owned(),
            tool_or_model: "identity_text_extract".to_owned(),
            tool_version: env!("CARGO_PKG_VERSION").to_owned(),
        }
    }

    pub fn execute(
        &self,
        request: &ProviderExecutionRequest,
    ) -> Result<ProviderExecutionOutcome, ApplicationError> {
        if request.pipeline_id.as_str() != "local_extract_text" {
            return Err(ApplicationError::capability_unavailable(
                request.pipeline_id.as_str(),
                "P5",
            ));
        }
        if request.input_text.trim().is_empty() {
            return Err(ApplicationError::Integrity(
                "local text extraction needs non-empty revision text".to_owned(),
            ));
        }
        if Sha256::of_bytes(request.input_text.as_bytes()) != request.input_sha256 {
            return Err(ApplicationError::Integrity(
                "local extraction input bytes do not match the queued C0 hash".to_owned(),
            ));
        }
        Ok(ProviderExecutionOutcome {
            task: ProviderTaskRef {
                provider: "local_extract".to_owned(),
                task_id: format!("local:{}", request.job_id),
            },
            kind: self.identity().kind,
            provider: self.identity().provider,
            tool_or_model: self.identity().tool_or_model,
            tool_version: self.identity().tool_version,
            content_text: Some(request.input_text.clone()),
            content_json: None,
            media_type: Some("text/plain".to_owned()),
            language: None,
            params: Metadata::parse(r#"{"mode":"identity_text_extract","text_rewritten":false}"#)
                .map_err(ApplicationError::from)?,
            usage: Metadata::empty(),
            loss_notes: Some("Exact C0 revision text; no rewrite or normalization.".to_owned()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use babata_domain::{JobId, PipelineId, RevisionId};

    #[test]
    fn local_extract_preserves_exact_text() {
        let text = "line one\nline two";
        let outcome = LocalExtractProvider
            .execute(&ProviderExecutionRequest {
                job_id: JobId::new(),
                pipeline_id: PipelineId::new("local_extract_text"),
                revision_id: RevisionId::new(),
                input_sha256: Sha256::of_bytes(text.as_bytes()),
                input_text: text.to_owned(),
            })
            .unwrap();
        assert_eq!(outcome.content_text.as_deref(), Some(text));
        assert_eq!(outcome.kind, DerivativeKind::ExtractedText);
    }
}
