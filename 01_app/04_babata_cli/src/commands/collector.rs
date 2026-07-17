#[derive(Debug, clap::Subcommand)]
pub enum CollectorCommand {
    Start { route: String },
    Candidates { session: String },
    Select { session: String },
    Status { session: String },
    Recollect { item: String },
}
