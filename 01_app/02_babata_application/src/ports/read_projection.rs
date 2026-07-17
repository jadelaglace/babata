use babata_domain::ItemId;

use crate::{ApplicationError, OperationStatus, RecordDetail, SearchPage, SearchQuery};

pub trait ReadProjectionPort {
    fn rebuild(&self) -> Result<OperationStatus, ApplicationError>;
    fn search(&self, query: SearchQuery) -> Result<SearchPage, ApplicationError>;
    fn show(&self, item_id: &ItemId) -> Result<RecordDetail, ApplicationError>;
    fn traverse(&self, item_id: &ItemId) -> Result<Vec<ItemId>, ApplicationError>;
    fn status(&self) -> Result<OperationStatus, ApplicationError>;
}
