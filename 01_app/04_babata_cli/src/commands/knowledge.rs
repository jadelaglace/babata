#[derive(Debug, clap::Subcommand)]
pub enum KnowledgeCommand {
    Review {
        #[arg(long)]
        item: String,
        #[arg(long)]
        revision: String,
    },
    Digest {
        #[arg(long)]
        item: String,
        #[arg(long)]
        revision: String,
    },
    Ingest {
        #[arg(long)]
        derivative: String,
    },
    RegisterFirstParty {
        #[arg(long)]
        item: String,
        #[arg(long)]
        revision: String,
        #[arg(long)]
        definition: std::path::PathBuf,
    },
    ShowEntry {
        #[arg(long)]
        semantic: String,
    },
    Score {
        #[arg(long)]
        semantic: String,
        #[arg(long)]
        interest: u8,
        #[arg(long)]
        strategy: u8,
        #[arg(long)]
        consensus: u8,
        #[arg(long)]
        rationale: String,
    },
    Show {
        #[arg(long)]
        suggestion: String,
    },
    ReviewSuggestion {
        #[arg(long)]
        suggestion: String,
        #[arg(long)]
        decision: SuggestionDecisionArg,
        #[arg(long)]
        reason: Option<String>,
        #[arg(long)]
        first_party_item: Option<String>,
        #[arg(long)]
        first_party_revision: Option<String>,
    },
    ListProfiles,
    CreateProfile {
        #[arg(long)]
        interest: u8,
        #[arg(long)]
        strategy: u8,
        #[arg(long)]
        consensus: u8,
        #[arg(long)]
        rationale: String,
    },
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum SuggestionDecisionArg {
    Accept,
    Modify,
    Reject,
}

impl From<SuggestionDecisionArg> for babata_domain::SuggestionDecisionKind {
    fn from(value: SuggestionDecisionArg) -> Self {
        match value {
            SuggestionDecisionArg::Accept => Self::Accept,
            SuggestionDecisionArg::Modify => Self::Modify,
            SuggestionDecisionArg::Reject => Self::Reject,
        }
    }
}
