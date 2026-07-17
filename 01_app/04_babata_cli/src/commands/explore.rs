#[derive(Debug, clap::Subcommand)]
pub enum ExploreCommand {
    Search { query: String },
    Show { item: String },
}
