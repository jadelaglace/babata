#[derive(Debug, clap::Subcommand)]
pub enum ProcessCommand {
    Enqueue { pipeline: String, revision: String },
    RunOnce,
    Status { job: String },
    Retry { job: String },
    Cancel { job: String },
    ListPipelines,
}
