#[derive(Debug, clap::Subcommand)]
pub enum KnowledgeCommand {
    Record {
        item: String,
    },
    Relate {
        from: String,
        to: String,
    },
    Classify {
        item: String,
        classification: String,
    },
    Model {
        item: String,
    },
    Score {
        item: String,
    },
    Analyze {
        item: String,
    },
    DecideSuggestion {
        suggestion: String,
        decision: String,
    },
}
