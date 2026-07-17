#[derive(Debug, clap::Subcommand)]
pub enum SublibrariesCommand {
    Create { title: String },
    Revise { sublibrary: String },
    Show { sublibrary: String },
    Materialize { sublibrary: String },
}
