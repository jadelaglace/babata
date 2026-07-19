use babata_application::{
    ApplicationError, CapabilityService, CaptureService, ProcessService, WorkspaceService,
};
use babata_domain::{PipelineId, RevisionId, RunId};
use babata_infrastructure::{
    FileAssetStore, StaticCapabilityRegistry, SystemClock, load_config, open_derived_database,
    open_raw_database, raw_status,
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
            | crate::commands::CaptureCommand::Export(_),
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
        RootCommand::Knowledge(_) => return Err(unavailable("knowledge", "P6")),
        RootCommand::Process(command) => {
            let repository =
                open_derived_database(&config.paths(), config.sqlite.busy_timeout_ms)?;
            let raw = open_raw_database(&config.paths(), config.sqlite.busy_timeout_ms)?;
            let assets = FileAssetStore::new(config.paths());
            let service = ProcessService::new(repository, raw, assets, SystemClock);
            match command {
                ProcessCommand::ListPipelines => {
                    let pipelines = service.list_pipelines()?;
                    render_value(&pipelines, cli.json)?;
                }
                ProcessCommand::Register { .. } => {
                    let register = crate::commands::process::build_register_command(
                        &command,
                        &FileAssetStore::new(config.paths()),
                    )
                    .map_err(|error| {
                        Box::new(ApplicationError::Integrity(error))
                            as Box<dyn std::error::Error>
                    })?;
                    let outcome = service.register_derivative(register)?;
                    if cli.json {
                        render_value(&outcome, true)?;
                    } else {
                        match &outcome.derivative_id {
                            Some(derivative_id) => println!("{} {}", outcome.run_id, derivative_id),
                            None => println!("{}", outcome.run_id),
                        }
                    }
                }
                ProcessCommand::RegisterFailure { .. } => {
                    let failure = crate::commands::process::build_failure_command(&command)
                        .map_err(|error| {
                            Box::new(ApplicationError::Integrity(error))
                                as Box<dyn std::error::Error>
                        })?;
                    let outcome = service.register_failure(failure)?;
                    if cli.json {
                        render_value(&outcome, true)?;
                    } else {
                        println!("{}", outcome.run_id);
                    }
                }
                ProcessCommand::ShowRun { run } => {
                    let run_id = RunId::parse(&run).map_err(|error| {
                        Box::new(ApplicationError::Domain(error)) as Box<dyn std::error::Error>
                    })?;
                    let outcome = service.show_run(&run_id)?;
                    render_value(&outcome, cli.json)?;
                }
                ProcessCommand::ListRuns { revision } => {
                    let revision_id = RevisionId::parse(&revision).map_err(|error| {
                        Box::new(ApplicationError::Domain(error)) as Box<dyn std::error::Error>
                    })?;
                    let runs = service.list_runs_for_revision(&revision_id)?;
                    render_value(&runs, cli.json)?;
                }
                ProcessCommand::Enqueue { pipeline, revision } => {
                    let outcome = service.enqueue(babata_application::EnqueueProcessCommand {
                        pipeline_id: PipelineId::new(pipeline),
                        revision_id: RevisionId::parse(&revision).map_err(|error| {
                            Box::new(ApplicationError::Domain(error)) as Box<dyn std::error::Error>
                        })?,
                    })?;
                    render_value(&outcome, cli.json)?;
                }
                ProcessCommand::RunOnce => {
                    let outcome = service.run_once()?;
                    render_value(&outcome, cli.json)?;
                }
                ProcessCommand::Status { job } => {
                    let job_id = babata_domain::JobId::parse(&job).map_err(|error| {
                        Box::new(ApplicationError::Domain(error)) as Box<dyn std::error::Error>
                    })?;
                    let outcome = service.status(&job_id)?;
                    render_value(&outcome, cli.json)?;
                }
                ProcessCommand::Retry { job } => {
                    let job_id = babata_domain::JobId::parse(&job).map_err(|error| {
                        Box::new(ApplicationError::Domain(error)) as Box<dyn std::error::Error>
                    })?;
                    let outcome = service.retry(&job_id)?;
                    render_value(&outcome, cli.json)?;
                }
                ProcessCommand::Cancel { job } => {
                    let job_id = babata_domain::JobId::parse(&job).map_err(|error| {
                        Box::new(ApplicationError::Domain(error)) as Box<dyn std::error::Error>
                    })?;
                    let outcome = service.cancel(&job_id)?;
                    render_value(&outcome, cli.json)?;
                }
            }
        }
        RootCommand::Explore(_) => return Err(unavailable("explore", "P6")),
        RootCommand::Sublibraries(_) => return Err(unavailable("sublibraries", "P6")),
        RootCommand::Views(_) => return Err(unavailable("views", "P6")),
        RootCommand::Outputs(_) => return Err(unavailable("outputs", "P6")),
        RootCommand::Routes(_) => return Err(unavailable("routes", "P4")),
        RootCommand::Ops(_) => return Err(unavailable("ops.backup", "P8")),
    }
    Ok(())
}

fn unavailable(capability: &str, phase: &str) -> Box<dyn std::error::Error> {
    Box::new(ApplicationError::capability_unavailable(capability, phase))
}

