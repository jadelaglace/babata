use babata_domain::{DerivativeKind, LogicalPath, Metadata, PipelineId, RevisionId, RunId, Sha256};

#[derive(Debug, clap::Subcommand)]
pub enum ProcessCommand {
    /// List pipelines available for registration / future execution.
    ListPipelines,
    /// Register a completed C1 derivative for a C0 revision (does not overwrite C0).
    Register {
        #[arg(long)]
        pipeline: String,
        #[arg(long)]
        revision: String,
        #[arg(long)]
        kind: String,
        #[arg(long)]
        provider: String,
        #[arg(long)]
        input_sha256: String,
        #[arg(long)]
        text_file: Option<String>,
        #[arg(long)]
        text: Option<String>,
        #[arg(long)]
        json_file: Option<String>,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        tool_version: Option<String>,
        #[arg(long)]
        language: Option<String>,
        #[arg(long)]
        loss_notes: Option<String>,
        #[arg(long)]
        retry_of: Option<String>,
        #[arg(long)]
        item: Option<String>,
    },
    /// Show a process run and its derivatives.
    ShowRun {
        #[arg(long)]
        run: String,
    },
    /// List process runs for a revision.
    ListRuns {
        #[arg(long)]
        revision: String,
    },
    Enqueue { pipeline: String, revision: String },
    RunOnce,
    Status { job: String },
    Retry { job: String },
    Cancel { job: String },
}

pub fn parse_kind(value: &str) -> Result<DerivativeKind, String> {
    match value {
        "extracted_text" => Ok(DerivativeKind::ExtractedText),
        "ocr_text" => Ok(DerivativeKind::OcrText),
        "transcript" => Ok(DerivativeKind::Transcript),
        "subtitle" => Ok(DerivativeKind::Subtitle),
        "summary" => Ok(DerivativeKind::Summary),
        "visual_description" => Ok(DerivativeKind::VisualDescription),
        "key_frame" => Ok(DerivativeKind::KeyFrame),
        "tags" => Ok(DerivativeKind::Tags),
        "structured_result" => Ok(DerivativeKind::StructuredResult),
        "media_metadata" => Ok(DerivativeKind::MediaMetadata),
        other => Err(format!("unknown derivative kind: {other}")),
    }
}

pub fn load_optional_file(path: &Option<String>) -> Result<Option<String>, String> {
    match path {
        Some(path) => std::fs::read_to_string(path)
            .map(Some)
            .map_err(|error| error.to_string()),
        None => Ok(None),
    }
}

pub fn build_register_command(
    command: &ProcessCommand,
) -> Result<babata_application::RegisterDerivativeCommand, String> {
    let ProcessCommand::Register {
        pipeline,
        revision,
        kind,
        provider,
        input_sha256,
        text_file,
        text,
        json_file,
        model,
        tool_version,
        language,
        loss_notes,
        retry_of,
        item,
    } = command
    else {
        return Err("not a register command".to_owned());
    };

    let content_text = if let Some(text) = text {
        Some(text.clone())
    } else {
        load_optional_file(text_file)?
    };
    let content_json = load_optional_file(json_file)?;

    Ok(babata_application::RegisterDerivativeCommand {
        pipeline_id: PipelineId::new(pipeline.clone()),
        revision_id: RevisionId::parse(revision).map_err(|e| e.to_string())?,
        item_id: item
            .as_ref()
            .map(|value| babata_domain::ItemId::parse(value).map_err(|e| e.to_string()))
            .transpose()?,
        input_sha256: Sha256::parse(input_sha256).map_err(|e| e.to_string())?,
        kind: parse_kind(kind)?,
        provider: provider.clone(),
        tool_or_model: model.clone(),
        tool_version: tool_version.clone(),
        retry_of_run_id: retry_of
            .as_ref()
            .map(|value| RunId::parse(value).map_err(|e| e.to_string()))
            .transpose()?,
        params: Metadata::empty(),
        usage: Metadata::empty(),
        loss_notes: loss_notes.clone(),
        content_text,
        content_json,
        logical_path: None::<LogicalPath>,
        media_type: None,
        language: language.clone(),
        input_asset_id: None,
        output_sha256: None,
        derivative_loss_notes: loss_notes.clone(),
        derivative_metadata: Metadata::empty(),
    })
}
