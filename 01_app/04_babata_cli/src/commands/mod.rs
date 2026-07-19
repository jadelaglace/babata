pub mod capabilities;
pub mod capture;
pub mod collector;
pub mod data;
pub mod explore;
pub mod knowledge;
pub mod ops;
pub mod outputs;
pub mod process;
pub mod routes;
pub mod sublibraries;
pub mod views;
pub mod workspace;

use babata_application::{CaptureService, WorkspaceService};
use babata_infrastructure::{FileAssetStore, SqliteRawRepository, SystemClock};
use clap::{Parser, Subcommand};

pub use capabilities::CapabilitiesCommand;
pub use capture::CaptureCommand;
pub use collector::CollectorCommand;
pub use data::DataCommand;
pub use explore::ExploreCommand;
pub use knowledge::KnowledgeCommand;
pub use ops::OpsCommand;
pub use outputs::OutputsCommand;
pub use process::ProcessCommand;
pub use routes::RoutesCommand;
pub use sublibraries::SublibrariesCommand;
pub use views::ViewsCommand;
pub use workspace::{AnnotateInput, NoteInput, ReviseInput, WorkspaceCommand};

#[derive(Debug, Parser)]
#[command(name = "babata")]
pub struct Cli {
    #[arg(long, global = true)]
    pub json: bool,
    #[command(subcommand)]
    pub command: RootCommand,
}

#[derive(Debug, Subcommand)]
pub enum RootCommand {
    Data {
        #[command(subcommand)]
        command: DataCommand,
    },
    #[command(subcommand)]
    Capabilities(CapabilitiesCommand),
    #[command(subcommand)]
    Collector(CollectorCommand),
    #[command(subcommand)]
    Capture(CaptureCommand),
    #[command(subcommand)]
    Workspace(WorkspaceCommand),
    Create(NoteInput),
    Revise(ReviseInput),
    Annotate(AnnotateInput),
    #[command(subcommand)]
    Knowledge(KnowledgeCommand),
    #[command(subcommand)]
    Process(Box<ProcessCommand>),
    #[command(subcommand)]
    Explore(ExploreCommand),
    #[command(subcommand)]
    Sublibraries(SublibrariesCommand),
    #[command(subcommand)]
    Views(ViewsCommand),
    #[command(subcommand)]
    Outputs(OutputsCommand),
    #[command(subcommand)]
    Routes(RoutesCommand),
    #[command(subcommand)]
    Ops(OpsCommand),
}

pub fn execute(
    command: RootCommand,
    capture: CaptureService<SqliteRawRepository, FileAssetStore, SystemClock>,
    workspace: WorkspaceService<SqliteRawRepository, FileAssetStore, SystemClock>,
) -> Result<capture::CaptureExecution, babata_application::ApplicationError> {
    capture::execute(command, capture, workspace)
}
