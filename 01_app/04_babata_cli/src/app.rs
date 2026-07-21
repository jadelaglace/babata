use babata_application::ports::ClockPort;
use babata_application::{
    ApplicationError, CapabilityService, CaptureService, CreateScoreProfileCommand,
    KnowledgeService, ProcessService, RecordRelevanceScoreCommand, RecordSuggestionReviewCommand,
    RegisterFirstPartySemanticCommand, SemanticDigestService, WorkspaceService,
};
use babata_domain::{
    DerivativeId, FirstPartySemanticDefinition, ItemId, PipelineId, RelevanceComponents,
    RevisionId, RunId, ScoreProfile, ScoreProfileId,
};
use babata_infrastructure::{
    AppConfig, FileAssetStore, StaticCapabilityRegistry, SystemClock, load_config,
    open_derived_database, open_job_database, open_knowledge_review_database, open_raw_database,
    processing::registry::ProcessProviderRouter, raw_status,
};
use clap::Parser;

use crate::{
    commands::{Cli, RootCommand, capture::CaptureExecution, process::ProcessCommand},
    render::{render_json, render_status, render_value},
};

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let config = load_config()?;
    match cli.command {
        RootCommand::Data {
            command: crate::commands::DataCommand::Status { json },
        } => render_status(
            &config,
            raw_status(&config.paths(), config.sqlite.busy_timeout_ms)?,
            cli.json || json,
        ),
        RootCommand::Capabilities(crate::commands::CapabilitiesCommand::List) => {
            let descriptors = CapabilityService::new(StaticCapabilityRegistry::default()).list()?;
            render_value(&descriptors, cli.json)?;
        }
        RootCommand::Collector(command) => {
            let outcome = crate::commands::collector::execute(command, &config)?;
            render_value(&outcome, cli.json)?;
        }
        command @ (RootCommand::Capture(
            crate::commands::CaptureCommand::Text(_)
            | crate::commands::CaptureCommand::File(_)
            | crate::commands::CaptureCommand::Export(_)
            | crate::commands::CaptureCommand::AttachAssets(_),
        )
        | RootCommand::Workspace(_)
        | RootCommand::Create(_)
        | RootCommand::Revise(_)
        | RootCommand::Annotate(_)) => {
            let repository = open_raw_database(&config.paths(), config.sqlite.busy_timeout_ms)?;
            let assets = FileAssetStore::new(config.paths());
            let outcome = crate::commands::execute(
                command,
                CaptureService::new(repository.clone(), assets.clone(), SystemClock),
                WorkspaceService::new(repository, assets, SystemClock),
            )?;
            match outcome {
                CaptureExecution::Single(outcome) if cli.json => render_json(&outcome)?,
                CaptureExecution::Single(outcome) => {
                    println!("{} {}", outcome.item_id, outcome.revision_id);
                }
                CaptureExecution::Batch(outcomes) if cli.json => render_value(&outcomes, true)?,
                CaptureExecution::Batch(outcomes) => {
                    for outcome in outcomes {
                        println!("{} {}", outcome.item_id, outcome.revision_id);
                    }
                }
            }
        }
        RootCommand::Capture(_) => return Err(unavailable("capture.provider", "P4")),
        RootCommand::Knowledge(command) => execute_knowledge(command, &config, cli.json)?,
        RootCommand::Process(command) => execute_process(*command, &config, cli.json)?,
        RootCommand::Explore(_) => return Err(unavailable("explore", "P6")),
        RootCommand::Sublibraries(_) => return Err(unavailable("sublibraries", "P6")),
        RootCommand::Views(_) => return Err(unavailable("views", "P6")),
        RootCommand::Outputs(_) => return Err(unavailable("outputs", "P6")),
        RootCommand::Routes(_) => return Err(unavailable("routes", "P4")),
        RootCommand::Ops(_) => return Err(unavailable("ops.backup", "P8")),
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn execute_knowledge(
    command: crate::commands::KnowledgeCommand,
    config: &AppConfig,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let raw = open_knowledge_review_database(&config.paths(), config.sqlite.busy_timeout_ms)?;
    let derived = open_derived_database(&config.paths(), config.sqlite.busy_timeout_ms)?;
    let assets = FileAssetStore::new(config.paths());
    let service = KnowledgeService::new(raw.clone(), derived.clone(), assets.clone());
    match command {
        crate::commands::KnowledgeCommand::Review { item, revision } => {
            render_value(
                &service.review(&ItemId::parse(item)?, &RevisionId::parse(revision)?)?,
                json,
            )?;
        }
        crate::commands::KnowledgeCommand::Digest { item, revision } => {
            let digest = SemanticDigestService::new(
                raw,
                derived,
                assets,
                babata_infrastructure::processing::semantic_digest::BailianSemanticDigestProvider::detect(),
                SystemClock,
            );
            render_value(
                &digest.digest(&ItemId::parse(item)?, &RevisionId::parse(revision)?)?,
                json,
            )?;
        }
        crate::commands::KnowledgeCommand::Ingest { derivative } => {
            render_value(
                &service.ingest_derivative(&DerivativeId::parse(derivative)?)?,
                json,
            )?;
        }
        crate::commands::KnowledgeCommand::RegisterFirstParty {
            item,
            revision,
            definition,
        } => {
            let definition: FirstPartySemanticDefinition =
                serde_json::from_str(&std::fs::read_to_string(definition)?)?;
            render_value(
                &service.register_first_party(&RegisterFirstPartySemanticCommand {
                    item_id: ItemId::parse(item)?,
                    revision_id: RevisionId::parse(revision)?,
                    definition,
                    author: "user".to_owned(),
                    created_at: SystemClock.now(),
                })?,
                json,
            )?;
        }
        crate::commands::KnowledgeCommand::ShowEntry { semantic } => {
            render_value(&service.show_entry(&semantic)?, json)?;
        }
        crate::commands::KnowledgeCommand::Score {
            semantic,
            interest,
            strategy,
            consensus,
            rationale,
        } => {
            render_value(
                &service.record_score(&RecordRelevanceScoreCommand {
                    semantic_id: semantic,
                    components: RelevanceComponents {
                        interest,
                        strategy,
                        consensus,
                        rationale,
                    },
                    author_kind: "first_party".to_owned(),
                    author: "user".to_owned(),
                    created_at: SystemClock.now(),
                })?,
                json,
            )?;
        }
        crate::commands::KnowledgeCommand::Show { suggestion } => {
            render_value(&service.show_semantic(&suggestion)?, json)?;
        }
        crate::commands::KnowledgeCommand::ReviewSuggestion {
            suggestion,
            decision,
            reason,
            first_party_item,
            first_party_revision,
        } => {
            let command = RecordSuggestionReviewCommand {
                suggestion_id: suggestion,
                decision: decision.into(),
                reason,
                first_party_item_id: first_party_item.map(ItemId::parse).transpose()?,
                first_party_revision_id: first_party_revision.map(RevisionId::parse).transpose()?,
                reviewer: "user".to_owned(),
                created_at: SystemClock.now(),
            };
            render_value(&service.review_suggestion(&command)?, json)?;
        }
        crate::commands::KnowledgeCommand::ListProfiles => {
            render_value(&service.score_profiles()?, json)?;
        }
        crate::commands::KnowledgeCommand::CreateProfile {
            interest,
            strategy,
            consensus,
            rationale,
        } => {
            let ordinal = u32::try_from(service.score_profiles()?.len())? + 1;
            let command = CreateScoreProfileCommand {
                profile: ScoreProfile {
                    profile_id: ScoreProfileId::new().to_string(),
                    ordinal,
                    interest_weight: interest,
                    strategy_weight: strategy,
                    consensus_weight: consensus,
                    rationale,
                    created_at: SystemClock.now(),
                },
                author_kind: "first_party".to_owned(),
                author: "user".to_owned(),
            };
            render_value(&service.create_score_profile(&command)?, json)?;
        }
    }
    Ok(())
}

fn execute_process(
    command: ProcessCommand,
    config: &AppConfig,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository = open_derived_database(&config.paths(), config.sqlite.busy_timeout_ms)?;
    let jobs = open_job_database(&config.paths(), config.sqlite.busy_timeout_ms)?;
    let raw = open_raw_database(&config.paths(), config.sqlite.busy_timeout_ms)?;
    let assets = FileAssetStore::new(config.paths());
    let service = ProcessService::new(repository, raw, assets, SystemClock)
        .with_runtime(jobs, ProcessProviderRouter::detect());
    match command {
        ProcessCommand::ListPipelines => render_value(&service.list_pipelines()?, json)?,
        ProcessCommand::Register { .. } => {
            let register =
                crate::commands::process::build_register_command(&command).map_err(|error| {
                    Box::new(ApplicationError::Integrity(error)) as Box<dyn std::error::Error>
                })?;
            let outcome = service.register_derivative(register)?;
            if json {
                render_value(&outcome, true)?;
            } else {
                match &outcome.derivative_id {
                    Some(derivative_id) => println!("{} {}", outcome.run_id, derivative_id),
                    None => println!("{}", outcome.run_id),
                }
            }
        }
        ProcessCommand::RegisterFailure { .. } => {
            let failure =
                crate::commands::process::build_failure_command(&command).map_err(|error| {
                    Box::new(ApplicationError::Integrity(error)) as Box<dyn std::error::Error>
                })?;
            let outcome = service.register_failure(failure)?;
            if json {
                render_value(&outcome, true)?;
            } else {
                println!("{}", outcome.run_id);
            }
        }
        ProcessCommand::ShowRun { run } => {
            let run_id = RunId::parse(&run).map_err(|error| {
                Box::new(ApplicationError::Domain(error)) as Box<dyn std::error::Error>
            })?;
            render_value(&service.show_run(&run_id)?, json)?;
        }
        ProcessCommand::ListRuns { revision } => {
            let revision_id = RevisionId::parse(&revision).map_err(|error| {
                Box::new(ApplicationError::Domain(error)) as Box<dyn std::error::Error>
            })?;
            render_value(&service.list_runs_for_revision(&revision_id)?, json)?;
        }
        ProcessCommand::DeleteResult { run, reason } => {
            let run_id = RunId::parse(&run).map_err(|error| {
                Box::new(ApplicationError::Domain(error)) as Box<dyn std::error::Error>
            })?;
            render_value(&service.delete_result(&run_id, &reason)?, json)?;
        }
        ProcessCommand::Enqueue { pipeline, revision } => {
            let outcome = service.enqueue(babata_application::EnqueueProcessCommand {
                pipeline_id: PipelineId::new(pipeline),
                revision_id: RevisionId::parse(&revision).map_err(|error| {
                    Box::new(ApplicationError::Domain(error)) as Box<dyn std::error::Error>
                })?,
            })?;
            render_value(&outcome, json)?;
        }
        ProcessCommand::RunOnce => render_value(&service.run_once()?, json)?,
        ProcessCommand::Status { job } => {
            let job_id = parse_job_id(&job)?;
            render_value(&service.status(&job_id)?, json)?;
        }
        ProcessCommand::Retry { job } => {
            let job_id = parse_job_id(&job)?;
            render_value(&service.retry(&job_id)?, json)?;
        }
        ProcessCommand::Cancel { job } => {
            let job_id = parse_job_id(&job)?;
            render_value(&service.cancel(&job_id)?, json)?;
        }
    }
    Ok(())
}

fn parse_job_id(value: &str) -> Result<babata_domain::JobId, Box<dyn std::error::Error>> {
    babata_domain::JobId::parse(value)
        .map_err(|error| Box::new(ApplicationError::Domain(error)) as Box<dyn std::error::Error>)
}

fn unavailable(capability: &str, phase: &str) -> Box<dyn std::error::Error> {
    Box::new(ApplicationError::capability_unavailable(capability, phase))
}
