#[derive(Debug, clap::Subcommand)]
pub enum OutputsCommand {
    List,
    Build {
        #[arg(long)]
        kind: String,
        #[arg(long = "record")]
        records: Vec<String>,
        #[arg(long)]
        sublibrary: Option<String>,
        #[arg(long, requires = "sublibrary")]
        sublibrary_version: Option<u32>,
        #[arg(long)]
        description: String,
        #[arg(long, default_value = "babata-default/v1")]
        template_version: String,
    },
    Status {
        #[arg(long)]
        output: String,
    },
    Verify {
        #[arg(long)]
        output: String,
    },
    Delete {
        #[arg(long)]
        output: String,
    },
    Rebuild {
        #[arg(long)]
        output: String,
    },
}
