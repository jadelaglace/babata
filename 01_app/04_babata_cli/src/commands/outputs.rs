#[derive(Debug, clap::Subcommand)]
pub enum OutputsCommand {
    List,
    Build { kind: String },
    Status { output: String },
    Verify { output: String },
}
