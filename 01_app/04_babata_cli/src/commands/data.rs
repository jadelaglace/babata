#[derive(Debug, clap::Subcommand)]
pub enum DataCommand {
    Status {
        #[arg(long)]
        json: bool,
    },
}
