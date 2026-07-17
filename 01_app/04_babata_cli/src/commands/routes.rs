#[derive(Debug, clap::Subcommand)]
pub enum RoutesCommand {
    List,
    Show { route: String },
    Evaluate { route: String },
    Collect { route: String, source: String },
}
