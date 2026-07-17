use babata_domain::ItemId;

use crate::{ApplicationError, RecordDetail, SearchPage, SearchQuery};

#[derive(Debug, Default, Clone, Copy)]
pub struct ExploreService;

impl ExploreService {
    pub fn search(&self, _query: SearchQuery) -> Result<SearchPage, ApplicationError> {
        unavailable()
    }

    pub fn show(&self, _item_id: &ItemId) -> Result<RecordDetail, ApplicationError> {
        unavailable()
    }
}

fn unavailable<T>() -> Result<T, ApplicationError> {
    Err(ApplicationError::capability_unavailable("explore", "P6"))
}
