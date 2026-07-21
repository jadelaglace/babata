use crate::{ApplicationError, DenseExpressionPreviewDocument, DenseExpressionPreviewOutcome};

pub trait DenseExpressionPreviewPort {
    fn build(
        &self,
        document: &DenseExpressionPreviewDocument,
    ) -> Result<DenseExpressionPreviewOutcome, ApplicationError>;
    fn verify(
        &self,
        semantic_id: &str,
        source_sha256: &babata_domain::Sha256,
    ) -> Result<DenseExpressionPreviewOutcome, ApplicationError>;
    fn delete(&self, semantic_id: &str) -> Result<DenseExpressionPreviewOutcome, ApplicationError>;
}
