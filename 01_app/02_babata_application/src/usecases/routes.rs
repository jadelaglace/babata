use babata_domain::{CandidateEnvelope, RouteCoverage, SourceRouteDescriptor, SourceRouteId};

use crate::{ApplicationError, RouteCollectCommand, ports::RawRepositoryPort};

#[derive(Debug, Clone)]
pub struct RouteService<R> {
    repository: R,
    descriptors: Vec<SourceRouteDescriptor>,
}

impl<R> RouteService<R>
where
    R: RawRepositoryPort,
{
    pub fn new(repository: R, descriptors: Vec<SourceRouteDescriptor>) -> Self {
        Self {
            repository,
            descriptors,
        }
    }

    pub fn list(&self) -> Result<Vec<SourceRouteDescriptor>, ApplicationError> {
        Ok(self.descriptors.clone())
    }

    pub fn show(&self, id: &SourceRouteId) -> Result<SourceRouteDescriptor, ApplicationError> {
        let descriptor = self
            .descriptors
            .iter()
            .find(|descriptor| descriptor.id == *id)
            .ok_or_else(|| ApplicationError::NotFound(format!("route: {}", id.0)))?;
        Ok(descriptor.clone())
    }

    pub fn evaluate(&self, id: &SourceRouteId) -> Result<RouteCoverage, ApplicationError> {
        let evidence = self.repository.route_evidence(&id.0)?;
        if evidence.is_empty() {
            return Ok(RouteCoverage {
                metadata: false,
                attachments: false,
                revisions: false,
                limitations: vec!["no authorised route evidence recorded".to_owned()],
            });
        }
        let mut limitations = evidence
            .iter()
            .flat_map(|evidence| evidence.coverage.limitations.clone())
            .collect::<Vec<_>>();
        limitations.sort();
        limitations.dedup();
        Ok(RouteCoverage {
            metadata: evidence.iter().any(|evidence| evidence.coverage.metadata),
            attachments: evidence
                .iter()
                .any(|evidence| evidence.coverage.attachments),
            revisions: evidence.iter().any(|evidence| evidence.reimported),
            limitations,
        })
    }

    pub fn collect(
        &self,
        _command: RouteCollectCommand,
    ) -> Result<CandidateEnvelope, ApplicationError> {
        unavailable()
    }
}

fn unavailable<T>() -> Result<T, ApplicationError> {
    Err(ApplicationError::capability_unavailable("routes", "P4"))
}
