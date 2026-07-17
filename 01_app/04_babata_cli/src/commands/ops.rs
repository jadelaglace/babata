#[derive(Debug, clap::Subcommand)]
pub enum OpsCommand {
    Status,
    Doctor,
    Backup,
    RestoreVerify { snapshot: String },
}
