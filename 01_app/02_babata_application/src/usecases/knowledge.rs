use babata_domain::{
    ItemId, KnowledgeRecord, ModelSuggestion, SuggestionDecision, SuggestionDecisionKind,
};

use crate::ApplicationError;

#[derive(Debug, Default, Clone, Copy)]
pub struct KnowledgeService;

impl KnowledgeService {
    pub fn record(&self, _record: KnowledgeRecord) -> Result<KnowledgeRecord, ApplicationError> {
        unavailable()
    }

    pub fn relate(
        &self,
        _from: &ItemId,
        _to: &ItemId,
    ) -> Result<KnowledgeRecord, ApplicationError> {
        unavailable()
    }

    pub fn classify(
        &self,
        _item_id: &ItemId,
        _classification: &str,
    ) -> Result<KnowledgeRecord, ApplicationError> {
        unavailable()
    }

    pub fn model(
        &self,
        _item_id: &ItemId,
        _model: &str,
    ) -> Result<KnowledgeRecord, ApplicationError> {
        unavailable()
    }

    pub fn score(
        &self,
        _item_id: &ItemId,
        _score: &str,
    ) -> Result<KnowledgeRecord, ApplicationError> {
        unavailable()
    }

    pub fn analyze(
        &self,
        _item_id: &ItemId,
        _analysis: &str,
    ) -> Result<KnowledgeRecord, ApplicationError> {
        unavailable()
    }

    pub fn decide_suggestion(
        &self,
        _suggestion: &ModelSuggestion,
        _decision: SuggestionDecisionKind,
    ) -> Result<SuggestionDecision, ApplicationError> {
        unavailable()
    }
}

fn unavailable<T>() -> Result<T, ApplicationError> {
    Err(ApplicationError::capability_unavailable("knowledge", "P6"))
}
