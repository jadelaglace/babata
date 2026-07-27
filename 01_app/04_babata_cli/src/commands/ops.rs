#[derive(Debug, clap::Subcommand)]
pub enum OpsCommand {
    Status,
    Doctor,
    Backup,
    RestoreVerify {
        snapshot: String,
        #[arg(long)]
        target: Option<String>,
    },
    VerifyRestored {
        snapshot: String,
        #[arg(long)]
        target: String,
    },
}
