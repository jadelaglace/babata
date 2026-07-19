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
        /// Logical path under `BABATA_DATA_HOME` for an already stored derivative file.
        #[arg(long)]
        logical_path: Option<String>,
        /// External derivative file to import into managed C1 storage
        /// (`02_derived/files/sha256/...`) before registration.
        #[arg(long)]
        output_file: Option<String>,
        #[arg(long)]
        media_type: Option<String>,
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
        /// C0 asset that this derivative was produced from (media inputs).
        #[arg(long)]
        input_asset_id: Option<String>,
        /// Extra run params as a JSON object (never include secrets).
        #[arg(long, default_value = "{}")]
        params_json: String,
    },
    /// Record a failed processing attempt so retries keep honest history.
    RegisterFailure {
        #[arg(long)]
        pipeline: String,
        #[arg(long)]
        revision: String,
        #[arg(long)]
        provider: String,
        #[arg(long)]
        input_sha256: String,
        #[arg(long)]
        error_code: String,
        #[arg(long)]
        error_message: Option<String>,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        tool_version: Option<String>,
        #[arg(long)]
        loss_notes: Option<String>,
        #[arg(long)]
        retry_of: Option<String>,
        #[arg(long)]
        item: Option<String>,
        #[arg(long)]
        input_asset_id: Option<String>,
        #[arg(long, default_value = "{}")]
        params_json: String,
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
    Enqueue {
        pipeline: String,
        revision: String,
    },
    RunOnce,
    Status {
        job: String,
    },
    Retry {
        job: String,
    },
    Cancel {
        job: String,
    },
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

pub fn load_optional_file(path: Option<&str>) -> Result<Option<String>, String> {
    match path {
        Some(path) => std::fs::read_to_string(path)
            .map(Some)
            .map_err(|error| error.to_string()),
        None => Ok(None),
    }
}

pub fn build_register_command(
    command: &ProcessCommand,
    assets: &dyn babata_application::ports::AssetStorePort,
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
        logical_path,
        output_file,
        media_type,
        model,
        tool_version,
        language,
        loss_notes,
        retry_of,
        item,
        input_asset_id,
        params_json,
    } = command
    else {
        return Err("not a register command".to_owned());
    };

    let content_text = if let Some(text) = text {
        Some(text.clone())
    } else {
        load_optional_file(text_file.as_deref())?
    };
    let content_json = load_optional_file(json_file.as_deref())?;

    let mut logical_path = logical_path
        .as_ref()
        .map(|path| LogicalPath::parse(path).map_err(|e| e.to_string()))
        .transpose()?;
    let mut output_sha256 = None;
    if let Some(output_file) = output_file {
        if logical_path.is_some() {
            return Err("use either --logical-path or --output-file, not both".to_owned());
        }
        let (imported_path, imported_hash) = assets
            .import_derived_file(output_file)
            .map_err(|error| error.to_string())?;
        logical_path = Some(imported_path);
        output_sha256 = Some(imported_hash);
    }

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
        params: Metadata::parse(params_json).map_err(|e| e.to_string())?,
        usage: Metadata::empty(),
        loss_notes: loss_notes.clone(),
        content_text,
        content_json,
        logical_path,
        media_type: media_type.clone(),
        language: language.clone(),
        input_asset_id: input_asset_id
            .as_ref()
            .map(|value| babata_domain::AssetId::parse(value).map_err(|e| e.to_string()))
            .transpose()?,
        output_sha256,
        derivative_loss_notes: loss_notes.clone(),
        derivative_metadata: Metadata::empty(),
    })
}

pub fn build_failure_command(
    command: &ProcessCommand,
) -> Result<babata_application::RegisterFailureCommand, String> {
    let ProcessCommand::RegisterFailure {
        pipeline,
        revision,
        provider,
        input_sha256,
        error_code,
        error_message,
        model,
        tool_version,
        loss_notes,
        retry_of,
        item,
        input_asset_id,
        params_json,
    } = command
    else {
        return Err("not a register-failure command".to_owned());
    };

    Ok(babata_application::RegisterFailureCommand {
        pipeline_id: PipelineId::new(pipeline.clone()),
        revision_id: RevisionId::parse(revision).map_err(|e| e.to_string())?,
        item_id: item
            .as_ref()
            .map(|value| babata_domain::ItemId::parse(value).map_err(|e| e.to_string()))
            .transpose()?,
        input_sha256: Sha256::parse(input_sha256).map_err(|e| e.to_string())?,
        provider: provider.clone(),
        tool_or_model: model.clone(),
        tool_version: tool_version.clone(),
        retry_of_run_id: retry_of
            .as_ref()
            .map(|value| RunId::parse(value).map_err(|e| e.to_string()))
            .transpose()?,
        params: Metadata::parse(params_json).map_err(|e| e.to_string())?,
        error_code: error_code.clone(),
        error_message: error_message.clone(),
        loss_notes: loss_notes.clone(),
        input_asset_id: input_asset_id
            .as_ref()
            .map(|value| babata_domain::AssetId::parse(value).map_err(|e| e.to_string()))
            .transpose()?,
    })
}
