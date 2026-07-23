#[derive(Debug, clap::Subcommand)]
pub enum SublibrariesCommand {
    Create {
        #[arg(long)]
        definition: String,
    },
    Revise {
        #[arg(long)]
        sublibrary: String,
        #[arg(long)]
        expected_version: u32,
        #[arg(long)]
        definition: String,
    },
    List,
    Versions {
        #[arg(long)]
        sublibrary: String,
    },
    Show {
        #[arg(long)]
        sublibrary: String,
        #[arg(long)]
        version: Option<u32>,
    },
    Materialize {
        #[arg(long)]
        sublibrary: String,
        #[arg(long)]
        version: Option<u32>,
    },
    Status {
        #[arg(long)]
        sublibrary: String,
        #[arg(long)]
        version: u32,
    },
    Verify {
        #[arg(long)]
        sublibrary: String,
        #[arg(long)]
        version: u32,
    },
    Delete {
        #[arg(long)]
        sublibrary: String,
        #[arg(long)]
        version: u32,
    },
    Rebuild {
        #[arg(long)]
        sublibrary: String,
        #[arg(long)]
        version: u32,
    },
}
