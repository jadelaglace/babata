use babata_domain::{ProjectionStatus, SearchRecordDetail};

use crate::{
    ApplicationError, ProjectionOperationOutcome, SearchPage, SearchQuery, SurfaceQuery,
    ports::ReadProjectionPort,
};

#[derive(Debug, Clone)]
pub struct ExploreService<P> {
    projection: P,
}

impl<P> ExploreService<P>
where
    P: ReadProjectionPort,
{
    pub fn new(projection: P) -> Self {
        Self { projection }
    }

    pub fn rebuild(&self) -> Result<ProjectionOperationOutcome, ApplicationError> {
        self.projection.rebuild()
    }

    pub fn delete(&self) -> Result<ProjectionOperationOutcome, ApplicationError> {
        self.projection.delete()
    }

    pub fn search(&self, query: SearchQuery) -> Result<SearchPage, ApplicationError> {
        self.projection.search(query)
    }

    pub fn surface(&self, query: SurfaceQuery) -> Result<SearchPage, ApplicationError> {
        self.projection.surface(query)
    }

    pub fn show(&self, record_id: &str) -> Result<SearchRecordDetail, ApplicationError> {
        self.projection.show(record_id)
    }

    pub fn traverse(&self, record_id: &str) -> Result<Vec<SearchRecordDetail>, ApplicationError> {
        self.projection.traverse(record_id)
    }

    pub fn status(&self) -> Result<ProjectionStatus, ApplicationError> {
        self.projection.status()
    }
}
