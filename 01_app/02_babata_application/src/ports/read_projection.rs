use babata_domain::{ProjectionStatus, SearchRecordDetail};

use crate::{ApplicationError, ProjectionOperationOutcome, SearchPage, SearchQuery, SurfaceQuery};

pub trait ReadProjectionPort {
    fn rebuild(&self) -> Result<ProjectionOperationOutcome, ApplicationError>;
    fn delete(&self) -> Result<ProjectionOperationOutcome, ApplicationError>;
    fn search(&self, query: SearchQuery) -> Result<SearchPage, ApplicationError>;
    fn surface(&self, query: SurfaceQuery) -> Result<SearchPage, ApplicationError>;
    fn show(&self, record_id: &str) -> Result<SearchRecordDetail, ApplicationError>;
    fn traverse(&self, record_id: &str) -> Result<Vec<SearchRecordDetail>, ApplicationError>;
    fn status(&self) -> Result<ProjectionStatus, ApplicationError>;
}
