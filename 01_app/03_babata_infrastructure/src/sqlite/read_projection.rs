use babata_application::{
    ApplicationError, OperationStatus, RecordDetail, SearchPage, SearchQuery,
    ports::ReadProjectionPort,
};
use babata_domain::ItemId;

#[derive(Debug, Default, Clone, Copy)]
pub struct SqliteReadProjection;

impl ReadProjectionPort for SqliteReadProjection {
    fn rebuild(&self) -> Result<OperationStatus, ApplicationError> {
        unavailable()
    }

    fn search(&self, _query: SearchQuery) -> Result<SearchPage, ApplicationError> {
        unavailable()
    }

    fn show(&self, _item_id: &ItemId) -> Result<RecordDetail, ApplicationError> {
        unavailable()
    }

    fn traverse(&self, _item_id: &ItemId) -> Result<Vec<ItemId>, ApplicationError> {
        unavailable()
    }

    fn status(&self) -> Result<OperationStatus, ApplicationError> {
        unavailable()
    }
}

fn unavailable<T>() -> Result<T, ApplicationError> {
    Err(ApplicationError::capability_unavailable(
        "read_projection",
        "P6",
    ))
}
