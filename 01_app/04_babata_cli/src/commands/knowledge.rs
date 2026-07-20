#[derive(Debug, clap::Subcommand)]
pub enum KnowledgeCommand {
    Review {
        #[arg(long)]
        item: String,
        #[arg(long)]
        revision: String,
    },
}
