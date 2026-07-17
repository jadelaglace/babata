#[derive(Debug, clap::Subcommand)]
pub enum ViewsCommand {
    List,
    Build { view: String },
}
