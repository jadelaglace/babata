use std::collections::{BTreeSet, HashSet};

use babata_domain::{
    OutputBuild, OutputDocument, OutputId, OutputInputRecord, OutputKind, OutputScope,
    OutputScoreProfileRef, OutputVerification, Sha256,
};

use crate::{
    ApplicationError, BuildOutputCommand,
    ports::{ClockPort, OutputBuilderPort, ReadProjectionPort, SublibraryDefinitionPort},
};

use super::sublibraries::{resolve_definition, resolve_record_details};

pub struct OutputService<D, P, B, C> {
    definitions: D,
    projection: P,
    builder: B,
    clock: C,
}

impl<D, P, B, C> OutputService<D, P, B, C>
where
    D: SublibraryDefinitionPort,
    P: ReadProjectionPort,
    B: OutputBuilderPort,
    C: ClockPort,
{
    pub fn new(definitions: D, projection: P, builder: B, clock: C) -> Self {
        Self {
            definitions,
            projection,
            builder,
            clock,
        }
    }

    pub fn list(&self) -> Vec<OutputKind> {
        self.builder.supported_kinds()
    }

    pub fn build(&self, command: BuildOutputCommand) -> Result<OutputBuild, ApplicationError> {
        if !self.builder.supported_kinds().contains(&command.kind) {
            return Err(ApplicationError::capability_unavailable(
                format!("outputs.{:?}", command.kind).to_lowercase(),
                "unplanned",
            ));
        }
        let document = self.document(
            OutputId::new(),
            command.kind,
            command.scope,
            command.template_version,
        )?;
        self.builder.build(&document)
    }

    pub fn rebuild(&self, output_id: &OutputId) -> Result<OutputBuild, ApplicationError> {
        let previous = self.builder.status(output_id)?;
        let document = self.document(
            output_id.clone(),
            previous.kind,
            previous.scope,
            previous.template_version,
        )?;
        self.builder.rebuild(&document)
    }

    pub fn status(&self, output_id: &OutputId) -> Result<OutputBuild, ApplicationError> {
        self.builder.status(output_id)
    }

    pub fn verify(&self, output_id: &OutputId) -> Result<OutputVerification, ApplicationError> {
        let previous = self.builder.status(output_id)?;
        let document = self.document_at(
            output_id.clone(),
            previous.kind,
            previous.scope,
            previous.template_version,
            previous.generated_at,
        )?;
        self.builder.verify(&document)
    }

    pub fn delete(&self, output_id: &OutputId) -> Result<OutputBuild, ApplicationError> {
        self.builder.delete(output_id)
    }

    fn document(
        &self,
        id: OutputId,
        kind: OutputKind,
        scope: OutputScope,
        template_version: String,
    ) -> Result<OutputDocument, ApplicationError> {
        self.document_at(id, kind, scope, template_version, self.clock.now())
    }

    fn document_at(
        &self,
        id: OutputId,
        kind: OutputKind,
        scope: OutputScope,
        template_version: String,
        generated_at: babata_domain::UtcTimestamp,
    ) -> Result<OutputDocument, ApplicationError> {
        validate_scope(&scope)?;
        if template_version.trim().is_empty() {
            return Err(ApplicationError::Integrity(
                "template version is required".to_owned(),
            ));
        }
        let record_ids = if let Some(sublibrary) = &scope.sublibrary {
            let definition = self
                .definitions
                .find(
                    &sublibrary.sublibrary_id,
                    Some(sublibrary.definition_version),
                )?
                .ok_or_else(|| ApplicationError::NotFound(sublibrary.sublibrary_id.to_string()))?;
            resolve_definition(&self.projection, definition, generated_at.clone())?
                .members
                .into_iter()
                .map(|member| member.record.record_id)
                .collect::<Vec<_>>()
        } else {
            scope.record_ids.clone()
        };
        let details = resolve_record_details(&self.projection, &record_ids)?;
        let mut limitations = BTreeSet::new();
        let records = details
            .into_iter()
            .map(|detail| {
                for limitation in &detail.record.limitations {
                    limitations.insert(format!("{}: {limitation}", detail.record.record_id));
                }
                let bytes = serde_json::to_vec(&detail)
                    .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
                Ok(OutputInputRecord {
                    detail,
                    input_sha256: Sha256::of_bytes(&bytes),
                })
            })
            .collect::<Result<Vec<_>, ApplicationError>>()?;
        let score_profiles = records
            .iter()
            .filter_map(|record| record.detail.record.score.as_ref())
            .map(|score| OutputScoreProfileRef {
                profile_id: score.profile_id.clone(),
                profile_ordinal: score.profile_ordinal,
                interest_weight: score.interest_weight,
                strategy_weight: score.strategy_weight,
                consensus_weight: score.consensus_weight,
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        Ok(OutputDocument {
            id,
            kind,
            scope,
            generated_at,
            builder_version: "babata-rust-output/v1".to_owned(),
            template_version,
            score_profiles,
            records,
            limitations: limitations.into_iter().collect(),
        })
    }
}

fn validate_scope(scope: &OutputScope) -> Result<(), ApplicationError> {
    if scope.description.trim().is_empty() {
        return Err(ApplicationError::Integrity(
            "output scope description is required".to_owned(),
        ));
    }
    if scope.sublibrary.is_some() != scope.record_ids.is_empty() {
        return Err(ApplicationError::Integrity(
            "output scope requires exactly one explicit record set or sublibrary version"
                .to_owned(),
        ));
    }
    if scope
        .sublibrary
        .as_ref()
        .is_some_and(|scope| scope.definition_version == 0)
    {
        return Err(ApplicationError::Integrity(
            "sublibrary output version must be positive".to_owned(),
        ));
    }
    let unique = scope.record_ids.iter().collect::<HashSet<_>>();
    if unique.len() != scope.record_ids.len() {
        return Err(ApplicationError::Integrity(
            "output record IDs must be unique".to_owned(),
        ));
    }
    Ok(())
}
