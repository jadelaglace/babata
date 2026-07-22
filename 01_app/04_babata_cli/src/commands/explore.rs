#[derive(Debug, clap::Subcommand)]
pub enum ExploreCommand {
    /// Rebuild the disposable C2 search projection from authoritative C0/C1.
    Rebuild,
    /// Delete only the disposable C2 search projection.
    Delete,
    /// Show projection readiness, source fingerprint, and record counts.
    Status,
    /// Search text and structured facets. Text is optional.
    Search(Box<SearchArgs>),
    /// Show one projected raw or semantic record with navigation evidence.
    Show { record: String },
    /// Follow all non-broken relations from one projected record.
    Traverse { record: String },
    /// Surface scored content with direction, relevance, time, and relation reasons.
    Surface(SurfaceArgs),
}

#[derive(Debug, clap::Args)]
pub struct SearchArgs {
    pub query: Option<String>,
    #[arg(long, value_parser = ["external", "first_party"])]
    pub source_kind: Option<String>,
    #[arg(long)]
    pub provider: Option<String>,
    #[arg(long, value_parser = [
        "text", "document", "image", "audio", "video", "web_page", "archive", "unknown"
    ])]
    pub content_type: Option<String>,
    #[arg(long)]
    pub from: Option<String>,
    #[arg(long)]
    pub to: Option<String>,
    #[arg(long, value_parser = ["map_direction", "knowledge", "case", "log", "insight"])]
    pub semantic_kind: Option<String>,
    #[arg(long, value_parser = ["knowledge_map", "knowledge_and_cases", "cognitive_trail"])]
    pub realm: Option<String>,
    #[arg(long)]
    pub state: Option<String>,
    #[arg(long, value_parser = ["accessible", "restricted", "inaccessible", "removed", "unknown"])]
    pub access_state: Option<String>,
    #[arg(long)]
    pub person: Option<String>,
    #[arg(long)]
    pub map_node: Option<String>,
    #[arg(long)]
    pub tag: Option<String>,
    #[arg(long)]
    pub relation_kind: Option<String>,
    #[arg(long)]
    pub related_to: Option<String>,
    #[arg(long)]
    pub processing_state: Option<String>,
    #[arg(long, value_parser = ["external", "first_party", "machine"])]
    pub origin_kind: Option<String>,
    #[arg(long, value_parser = ["first_party", "unreviewed", "accepted", "modified", "rejected"])]
    pub review_state: Option<String>,
    #[arg(long)]
    pub restricted: Option<bool>,
    #[arg(long)]
    pub missing: Option<bool>,
    #[arg(long)]
    pub media_only: Option<bool>,
    #[arg(long)]
    pub attachment_only: Option<bool>,
    #[arg(long)]
    pub profile: Option<String>,
    #[arg(long)]
    pub min_interest: Option<u8>,
    #[arg(long)]
    pub min_strategy: Option<u8>,
    #[arg(long)]
    pub min_consensus: Option<u8>,
    #[arg(long)]
    pub min_weighted_score: Option<u16>,
    #[arg(long, default_value = "relevance", value_parser = [
        "relevance", "newest", "interest", "strategy", "consensus", "weighted_score"
    ])]
    pub sort: String,
    #[arg(long, default_value_t = 20)]
    pub limit: u32,
    #[arg(long)]
    pub cursor: Option<String>,
}

#[derive(Debug, clap::Args)]
pub struct SurfaceArgs {
    #[arg(long)]
    pub profile: Option<String>,
    #[arg(long)]
    pub map_node: Option<String>,
    #[arg(long)]
    pub related_to: Option<String>,
    #[arg(long)]
    pub since: Option<String>,
    #[arg(long, default_value_t = 20)]
    pub limit: u32,
}
