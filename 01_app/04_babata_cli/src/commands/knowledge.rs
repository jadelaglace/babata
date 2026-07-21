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
    CreateMapNode {
        #[arg(long)]
        level: MapNodeLevelArg,
        #[arg(long)]
        name: String,
        #[arg(long = "parent")]
        parents: Vec<String>,
        #[arg(long = "tag")]
        tags: Vec<String>,
        #[arg(long)]
        rationale: String,
    },
    RenameMapNode {
        #[arg(long = "map-node")]
        map_node: String,
        #[arg(long)]
        name: String,
        #[arg(long)]
        rationale: String,
    },
    DeactivateMapNode {
        #[arg(long = "map-node")]
        map_node: String,
        #[arg(long)]
        rationale: String,
    },
    MergeMapNode {
        #[arg(long = "map-node")]
        map_node: String,
        #[arg(long = "into")]
        target: String,
        #[arg(long)]
        rationale: String,
    },
    ChangeMapParent {
        #[arg(long)]
        parent: String,
        #[arg(long)]
        child: String,
        #[arg(long)]
        change: AssignmentChangeArg,
        #[arg(long)]
        rationale: String,
    },
    ChangeMapAssignment {
        #[arg(long)]
        semantic: String,
        #[arg(long = "map-node")]
        map_node: String,
        #[arg(long)]
        change: AssignmentChangeArg,
        #[arg(long)]
        rationale: String,
    },
    TagMapNode {
        #[arg(long = "map-node")]
        map_node: String,
        #[arg(long)]
        tag: String,
        #[arg(long)]
        change: AssignmentChangeArg,
        #[arg(long)]
        rationale: String,
    },
    ShowMapNode {
        #[arg(long = "map-node")]
        map_node: String,
    },
    BuildDensePreview {
        #[arg(long)]
        semantic: String,
    },
    VerifyDensePreview {
        #[arg(long)]
        semantic: String,
    },
    DeleteDensePreview {
        #[arg(long)]
        semantic: String,
    },
    Score {
        #[arg(
            long,
            required_unless_present = "map_node",
            conflicts_with = "map_node"
        )]
        semantic: Option<String>,
        #[arg(
            long = "map-node",
            required_unless_present = "semantic",
            conflicts_with = "semantic"
        )]
        map_node: Option<String>,
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
pub enum MapNodeLevelArg {
    Discipline,
    Branch,
}

impl From<MapNodeLevelArg> for babata_domain::MapNodeLevel {
    fn from(value: MapNodeLevelArg) -> Self {
        match value {
            MapNodeLevelArg::Discipline => Self::Discipline,
            MapNodeLevelArg::Branch => Self::Branch,
        }
    }
}

#[derive(Debug, Clone, Copy, clap::ValueEnum)]
pub enum AssignmentChangeArg {
    Assign,
    Unassign,
}

impl From<AssignmentChangeArg> for babata_application::AssignmentChange {
    fn from(value: AssignmentChangeArg) -> Self {
        match value {
            AssignmentChangeArg::Assign => Self::Assign,
            AssignmentChangeArg::Unassign => Self::Unassign,
        }
    }
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
