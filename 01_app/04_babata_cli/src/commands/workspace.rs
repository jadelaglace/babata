#[derive(Debug, clap::Subcommand)]
pub enum WorkspaceCommand {
    Create(NoteInput),
    Revise(ReviseInput),
    Annotate(AnnotateInput),
}

#[derive(Debug, clap::Args)]
pub struct NoteInput {
    #[arg(long)]
    pub context: Option<String>,
    #[arg(long)]
    pub text: Option<String>,
    #[arg(long)]
    pub path: Option<std::path::PathBuf>,
    #[arg(long, default_value = "{}")]
    pub metadata_json: String,
}

#[derive(Debug, clap::Args)]
pub struct ReviseInput {
    #[arg(long)]
    pub parent: String,
    #[command(flatten)]
    pub note: NoteInput,
    #[arg(long)]
    pub note_text: Option<String>,
}

#[derive(Debug, clap::Args)]
pub struct AnnotateInput {
    #[arg(long)]
    pub target: String,
    #[command(flatten)]
    pub note: NoteInput,
}
