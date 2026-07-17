use babata_domain::{
    CandidateSummary, CollectionItemState, CollectionSelection, CollectionSessionId, ItemId,
    RecollectionState, SourceRouteId,
};

use crate::ApplicationError;

#[derive(Debug, Default, Clone, Copy)]
pub struct CollectorSessionService;

impl CollectorSessionService {
    pub fn start(&self, _route_id: SourceRouteId) -> Result<CollectionSessionId, ApplicationError> {
        unavailable()
    }

    pub fn candidates(
        &self,
        _session_id: &CollectionSessionId,
    ) -> Result<Vec<CandidateSummary>, ApplicationError> {
        unavailable()
    }

    pub fn select(
        &self,
        _selection: CollectionSelection,
    ) -> Result<Vec<CollectionItemState>, ApplicationError> {
        unavailable()
    }

    pub fn status(
        &self,
        _session_id: &CollectionSessionId,
    ) -> Result<Vec<CollectionItemState>, ApplicationError> {
        unavailable()
    }

    pub fn recollect(&self, _item_id: &ItemId) -> Result<RecollectionState, ApplicationError> {
        unavailable()
    }
}

fn unavailable<T>() -> Result<T, ApplicationError> {
    Err(ApplicationError::capability_unavailable("collector", "P4"))
}
