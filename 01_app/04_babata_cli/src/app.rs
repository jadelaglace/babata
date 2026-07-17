use babata_application::{
    ApplicationError, CapabilityService, CaptureService, RouteService, WorkspaceService,
};
use babata_domain::SourceRouteId;
use babata_infrastructure::{
    FileAssetStore, StaticCapabilityRegistry, SystemClock, load_config, open_raw_database,
    raw_status, sources::registry,
};
use clap::Parser;

use crate::{
    commands::{Cli, RootCommand, capture::CaptureExecution},
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
            let mut descriptors =
                CapabilityService::new(StaticCapabilityRegistry::default()).list()?;
            if raw_status(&config.paths(), config.sqlite.busy_timeout_ms)?.reachable {
                let repository = open_raw_database(&config.paths(), config.sqlite.busy_timeout_ms)?;
                let routes = RouteService::new(repository, registry::descriptors()).list()?;
                for descriptor in &mut descriptors {
                    if let Some(route) = routes.iter().find(|route| route.id.0 == descriptor.id.0) {
                        descriptor.status = route.status;
                        descriptor.reason =
                            (route.status == babata_domain::CapabilityStatus::Disabled).then(
                                || "awaiting authorised contextual collection evidence".to_owned(),
                            );
                    }
                }
            }
            render_value(&descriptors, cli.json)?;
        }
        command @ (RootCommand::Capture(_)
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
        RootCommand::Collector(_) => return Err(unavailable("collector", "P4")),
        RootCommand::Knowledge(_) => return Err(unavailable("knowledge", "P6")),
        RootCommand::Process(_) => return Err(unavailable("processing", "P5")),
        RootCommand::Explore(_) => return Err(unavailable("explore", "P6")),
        RootCommand::Sublibraries(_) => return Err(unavailable("sublibraries", "P6")),
        RootCommand::Views(_) => return Err(unavailable("views", "P6")),
        RootCommand::Outputs(_) => return Err(unavailable("outputs", "P6")),
        RootCommand::Routes(command) => {
            let repository = open_raw_database(&config.paths(), config.sqlite.busy_timeout_ms)?;
            let routes = RouteService::new(repository, registry::descriptors());
            match command {
                crate::commands::RoutesCommand::List => render_value(&routes.list()?, cli.json)?,
                crate::commands::RoutesCommand::Show { route } => {
                    render_value(&routes.show(&SourceRouteId(route))?, cli.json)?;
                }
                crate::commands::RoutesCommand::Evaluate { route } => {
                    render_value(&routes.evaluate(&SourceRouteId(route))?, cli.json)?;
                }
                crate::commands::RoutesCommand::Collect { .. } => {
                    return Err(unavailable("routes.collect", "P4"));
                }
            }
        }
        RootCommand::Ops(_) => return Err(unavailable("ops.backup", "P8")),
    }
    Ok(())
}

fn unavailable(capability: &str, phase: &str) -> Box<dyn std::error::Error> {
    Box::new(ApplicationError::capability_unavailable(capability, phase))
}
