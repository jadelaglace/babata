use babata_application::ports::ClockPort;
use babata_application::{
    ApplicationError, BuildOutputCommand, CapabilityService, CaptureService,
    ChangeMapNodeTagCommand, ChangeMapParentCommand, ChangeSemanticMapAssignmentCommand,
    CreateMapNodeCommand, CreateScoreProfileCommand, CreateSublibraryCommand,
    DenseExpressionPreviewService, EvolveMapNodeAction, EvolveMapNodeCommand, ExploreService,
    KnowledgeService, OutputService, ProcessService, RecordRelevanceScoreCommand,
    RecordSuggestionReviewCommand, RegisterFirstPartySemanticCommand, ReviseSublibraryCommand,
    SearchQuery, SemanticDigestService, SublibraryService, SurfaceQuery, WorkspaceService,
};
use babata_domain::{
    DerivativeId, FirstPartySemanticDefinition, ItemId, OutputId, OutputScope, PageCursor,
    PipelineId, QueryFilter, RelevanceComponents, RelevanceTargetKind, RevisionId, RunId,
    ScoreProfile, ScoreProfileId, SublibraryId, SublibraryOutputScope, UtcTimestamp,
};
use babata_infrastructure::{
    AppConfig, DenseExpressionViewStore, FileAssetStore, OutputViewStore, SqliteReadProjection,
    StaticCapabilityRegistry, SublibraryViewStore, SystemClock, load_config, open_derived_database,
    open_job_database, open_knowledge_review_database, open_raw_database,
    processing::registry::ProcessProviderRouter, raw_status,
};
use clap::Parser;

use crate::{
    commands::{Cli, RootCommand, capture::CaptureExecution, process::ProcessCommand},
    render::{render_json, render_status, render_value},
};

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let config = load_config()?;
    match cli.command {
        RootCommand::Data {
            command: crate::commands::DataCommand::Status { json },
        } => render_status(
            &config,
            raw_status(&config.paths(), config.sqlite.busy_timeout_ms)?,
            cli.json || json,
        ),
        RootCommand::Capabilities(crate::commands::CapabilitiesCommand::List) => {
            let descriptors = CapabilityService::new(StaticCapabilityRegistry::default()).list()?;
            render_value(&descriptors, cli.json)?;
        }
        RootCommand::Collector(command) => {
            let outcome = crate::commands::collector::execute(command, &config)?;
            render_value(&outcome, cli.json)?;
        }
        command @ (RootCommand::Capture(
            crate::commands::CaptureCommand::Text(_)
            | crate::commands::CaptureCommand::File(_)
            | crate::commands::CaptureCommand::Export(_)
            | crate::commands::CaptureCommand::AttachAssets(_),
        )
        | RootCommand::Workspace(_)
        | RootCommand::Create(_)
        | RootCommand::Revise(_)
        | RootCommand::Annotate(_)) => {
            let repository = open_raw_database(&config.paths(), config.sqlite.busy_timeout_ms)?;
            let assets = FileAssetStore::new(config.paths());
            let outcome = crate::commands::execute(
                command,
                CaptureService::new(repository.clone(), assets.clone(), SystemClock),
                WorkspaceService::new(repository, assets, SystemClock),
            )?;
            match outcome {
                CaptureExecution::Single(outcome) if cli.json => render_json(&outcome)?,
                CaptureExecution::Single(outcome) => {
                    println!("{} {}", outcome.item_id, outcome.revision_id);
                }
                CaptureExecution::Batch(outcomes) if cli.json => render_value(&outcomes, true)?,
                CaptureExecution::Batch(outcomes) => {
                    for outcome in outcomes {
                        println!("{} {}", outcome.item_id, outcome.revision_id);
                    }
                }
            }
        }
        RootCommand::Capture(_) => return Err(unavailable("capture.provider", "P4")),
        RootCommand::Knowledge(command) => execute_knowledge(command, &config, cli.json)?,
        RootCommand::Process(command) => execute_process(*command, &config, cli.json)?,
        RootCommand::Explore(command) => execute_explore(*command, &config, cli.json)?,
        RootCommand::Sublibraries(command) => execute_sublibraries(command, &config, cli.json)?,
        RootCommand::Views(_) => return Err(unavailable("views", "P6")),
        RootCommand::Outputs(command) => execute_outputs(command, &config, cli.json)?,
        RootCommand::Routes(_) => return Err(unavailable("routes", "P4")),
        RootCommand::Ops(_) => return Err(unavailable("ops.backup", "P8")),
    }
    Ok(())
}

fn execute_sublibraries(
    command: crate::commands::SublibrariesCommand,
    config: &AppConfig,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository = open_raw_database(&config.paths(), config.sqlite.busy_timeout_ms)?;
    let service = SublibraryService::new(
        repository.clone(),
        FileAssetStore::new(config.paths()),
        SystemClock,
        repository,
        SqliteReadProjection::new(config.paths(), config.sqlite.busy_timeout_ms),
        SublibraryViewStore::new(config.paths()),
    );
    match command {
        crate::commands::SublibrariesCommand::Create { definition } => {
            render_value(
                &service.create(CreateSublibraryCommand {
                    definition: read_json(&definition)?,
                    author: "user".to_owned(),
                })?,
                json,
            )?;
        }
        crate::commands::SublibrariesCommand::Revise {
            sublibrary,
            expected_version,
            definition,
        } => {
            render_value(
                &service.revise(ReviseSublibraryCommand {
                    sublibrary_id: SublibraryId::parse(sublibrary)?,
                    expected_version,
                    definition: read_json(&definition)?,
                    author: "user".to_owned(),
                })?,
                json,
            )?;
        }
        crate::commands::SublibrariesCommand::List => render_value(&service.list()?, json)?,
        crate::commands::SublibrariesCommand::Versions { sublibrary } => {
            render_value(&service.versions(&SublibraryId::parse(sublibrary)?)?, json)?;
        }
        crate::commands::SublibrariesCommand::Show {
            sublibrary,
            version,
        } => render_value(
            &service.show(&SublibraryId::parse(sublibrary)?, version)?,
            json,
        )?,
        crate::commands::SublibrariesCommand::Materialize {
            sublibrary,
            version,
        } => render_value(
            &service.materialize(&SublibraryId::parse(sublibrary)?, version)?,
            json,
        )?,
        crate::commands::SublibrariesCommand::Status {
            sublibrary,
            version,
        } => render_value(
            &service.materialization_status(&SublibraryId::parse(sublibrary)?, version)?,
            json,
        )?,
        crate::commands::SublibrariesCommand::Verify {
            sublibrary,
            version,
        } => render_value(
            &service.verify_materialization(&SublibraryId::parse(sublibrary)?, version)?,
            json,
        )?,
        crate::commands::SublibrariesCommand::Delete {
            sublibrary,
            version,
        } => render_value(
            &service.delete_materialization(&SublibraryId::parse(sublibrary)?, version)?,
            json,
        )?,
        crate::commands::SublibrariesCommand::Rebuild {
            sublibrary,
            version,
        } => render_value(
            &service.materialize(&SublibraryId::parse(sublibrary)?, Some(version))?,
            json,
        )?,
    }
    Ok(())
}

fn execute_outputs(
    command: crate::commands::OutputsCommand,
    config: &AppConfig,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let definitions = open_raw_database(&config.paths(), config.sqlite.busy_timeout_ms)?;
    let service = OutputService::new(
        definitions,
        SqliteReadProjection::new(config.paths(), config.sqlite.busy_timeout_ms),
        OutputViewStore::new(config.paths()),
        SystemClock,
    );
    match command {
        crate::commands::OutputsCommand::List => render_value(&service.list(), json)?,
        crate::commands::OutputsCommand::Build {
            kind,
            records,
            sublibrary,
            sublibrary_version,
            description,
            template_version,
        } => {
            let sublibrary = sublibrary
                .map(|id| {
                    Ok::<_, ApplicationError>(SublibraryOutputScope {
                        sublibrary_id: SublibraryId::parse(id)?,
                        definition_version: sublibrary_version.ok_or_else(|| {
                            ApplicationError::Integrity(
                                "--sublibrary-version is required with --sublibrary".to_owned(),
                            )
                        })?,
                    })
                })
                .transpose()?;
            render_value(
                &service.build(BuildOutputCommand {
                    kind: parse_wire(&kind)?,
                    scope: OutputScope {
                        record_ids: records,
                        sublibrary,
                        description,
                    },
                    template_version,
                })?,
                json,
            )?;
        }
        crate::commands::OutputsCommand::Status { output } => {
            render_value(&service.status(&OutputId::parse(output)?)?, json)?;
        }
        crate::commands::OutputsCommand::Verify { output } => {
            render_value(&service.verify(&OutputId::parse(output)?)?, json)?;
        }
        crate::commands::OutputsCommand::Delete { output } => {
            render_value(&service.delete(&OutputId::parse(output)?)?, json)?;
        }
        crate::commands::OutputsCommand::Rebuild { output } => {
            render_value(&service.rebuild(&OutputId::parse(output)?)?, json)?;
        }
    }
    Ok(())
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, Box<dyn std::error::Error>> {
    Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
}

fn execute_explore(
    command: crate::commands::ExploreCommand,
    config: &AppConfig,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let service = ExploreService::new(SqliteReadProjection::new(
        config.paths(),
        config.sqlite.busy_timeout_ms,
    ));
    match command {
        crate::commands::ExploreCommand::Rebuild => {
            render_value(&service.rebuild()?, json)?;
        }
        crate::commands::ExploreCommand::Delete => {
            render_value(&service.delete()?, json)?;
        }
        crate::commands::ExploreCommand::Status => {
            render_value(&service.status()?, json)?;
        }
        crate::commands::ExploreCommand::Show { record } => {
            render_value(&service.show(&record)?, json)?;
        }
        crate::commands::ExploreCommand::Traverse { record } => {
            render_value(&service.traverse(&record)?, json)?;
        }
        crate::commands::ExploreCommand::Search(args) => {
            let query = SearchQuery {
                filter: QueryFilter {
                    text: args.query,
                    source_kind: args.source_kind.as_deref().map(parse_wire).transpose()?,
                    provider: args.provider,
                    content_type: args.content_type.as_deref().map(parse_wire).transpose()?,
                    captured_from: args.from.as_deref().map(UtcTimestamp::parse).transpose()?,
                    captured_to: args.to.as_deref().map(UtcTimestamp::parse).transpose()?,
                    semantic_kind: args.semantic_kind.as_deref().map(parse_wire).transpose()?,
                    realm: args.realm.as_deref().map(parse_wire).transpose()?,
                    state: args.state,
                    access_state: args.access_state,
                    person: args.person,
                    map_node: args.map_node,
                    tag: args.tag,
                    relation_kind: args.relation_kind,
                    related_to: args.related_to,
                    processing_state: args.processing_state,
                    origin_kind: args.origin_kind,
                    review_state: args.review_state,
                    restricted: args.restricted,
                    missing: args.missing,
                    media_only: args.media_only,
                    attachment_only: args.attachment_only,
                    profile_id: args.profile,
                    min_interest: args.min_interest,
                    min_strategy: args.min_strategy,
                    min_consensus: args.min_consensus,
                    min_weighted_score: args.min_weighted_score,
                    sort: parse_wire(&args.sort)?,
                    limit: args.limit,
                },
                cursor: args.cursor.map(PageCursor),
            };
            render_value(&service.search(query)?, json)?;
        }
        crate::commands::ExploreCommand::Surface(args) => {
            render_value(
                &service.surface(SurfaceQuery {
                    profile_id: args.profile,
                    map_node: args.map_node,
                    related_to: args.related_to,
                    since: args.since.as_deref().map(UtcTimestamp::parse).transpose()?,
                    limit: args.limit,
                })?,
                json,
            )?;
        }
    }
    Ok(())
}

fn parse_wire<T: serde::de::DeserializeOwned>(value: &str) -> Result<T, ApplicationError> {
    serde_json::from_value(serde_json::Value::String(value.to_owned())).map_err(|error| {
        ApplicationError::Integrity(format!("invalid command value {value}: {error}"))
    })
}

#[allow(clippy::too_many_lines)]
fn execute_knowledge(
    command: crate::commands::KnowledgeCommand,
    config: &AppConfig,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let raw = open_knowledge_review_database(&config.paths(), config.sqlite.busy_timeout_ms)?;
    let derived = open_derived_database(&config.paths(), config.sqlite.busy_timeout_ms)?;
    let assets = FileAssetStore::new(config.paths());
    let service = KnowledgeService::new(raw.clone(), derived.clone(), assets.clone());
    let preview_service = DenseExpressionPreviewService::new(
        raw.clone(),
        DenseExpressionViewStore::new(config.paths()),
    );
    match command {
        crate::commands::KnowledgeCommand::Review { item, revision } => {
            render_value(
                &service.review(&ItemId::parse(item)?, &RevisionId::parse(revision)?)?,
                json,
            )?;
        }
        crate::commands::KnowledgeCommand::Digest { item, revision } => {
            let digest = SemanticDigestService::new(
                raw,
                derived,
                assets,
                babata_infrastructure::processing::semantic_digest::BailianSemanticDigestProvider::detect(),
                SystemClock,
            );
            render_value(
                &digest.digest(&ItemId::parse(item)?, &RevisionId::parse(revision)?)?,
                json,
            )?;
        }
        crate::commands::KnowledgeCommand::Ingest { derivative } => {
            render_value(
                &service.ingest_derivative(&DerivativeId::parse(derivative)?)?,
                json,
            )?;
        }
        crate::commands::KnowledgeCommand::RegisterFirstParty {
            item,
            revision,
            definition,
        } => {
            let definition: FirstPartySemanticDefinition =
                serde_json::from_str(&std::fs::read_to_string(definition)?)?;
            render_value(
                &service.register_first_party(&RegisterFirstPartySemanticCommand {
                    item_id: ItemId::parse(item)?,
                    revision_id: RevisionId::parse(revision)?,
                    definition,
                    author: "user".to_owned(),
                    created_at: SystemClock.now(),
                })?,
                json,
            )?;
        }
        crate::commands::KnowledgeCommand::ShowEntry { semantic } => {
            render_value(&service.show_entry(&semantic)?, json)?;
        }
        crate::commands::KnowledgeCommand::CreateMapNode {
            level,
            name,
            parents,
            tags,
            rationale,
        } => {
            render_value(
                &service.create_map_node(&CreateMapNodeCommand {
                    level: level.into(),
                    name,
                    parent_node_ids: parents,
                    tags,
                    rationale,
                    author_kind: "first_party".to_owned(),
                    author: "user".to_owned(),
                    created_at: SystemClock.now(),
                })?,
                json,
            )?;
        }
        crate::commands::KnowledgeCommand::RenameMapNode {
            map_node,
            name,
            rationale,
        } => {
            render_value(
                &service.evolve_map_node(&EvolveMapNodeCommand {
                    map_node_id: map_node,
                    action: EvolveMapNodeAction::Rename { name },
                    rationale,
                    author_kind: "first_party".to_owned(),
                    author: "user".to_owned(),
                    created_at: SystemClock.now(),
                })?,
                json,
            )?;
        }
        crate::commands::KnowledgeCommand::DeactivateMapNode {
            map_node,
            rationale,
        } => {
            render_value(
                &service.evolve_map_node(&EvolveMapNodeCommand {
                    map_node_id: map_node,
                    action: EvolveMapNodeAction::Deactivate,
                    rationale,
                    author_kind: "first_party".to_owned(),
                    author: "user".to_owned(),
                    created_at: SystemClock.now(),
                })?,
                json,
            )?;
        }
        crate::commands::KnowledgeCommand::MergeMapNode {
            map_node,
            target,
            rationale,
        } => {
            render_value(
                &service.evolve_map_node(&EvolveMapNodeCommand {
                    map_node_id: map_node,
                    action: EvolveMapNodeAction::Merge {
                        target_map_node_id: target,
                    },
                    rationale,
                    author_kind: "first_party".to_owned(),
                    author: "user".to_owned(),
                    created_at: SystemClock.now(),
                })?,
                json,
            )?;
        }
        crate::commands::KnowledgeCommand::ChangeMapParent {
            parent,
            child,
            change,
            rationale,
        } => {
            render_value(
                &service.change_map_parent(&ChangeMapParentCommand {
                    parent_map_node_id: parent,
                    child_map_node_id: child,
                    change: change.into(),
                    rationale,
                    author_kind: "first_party".to_owned(),
                    author: "user".to_owned(),
                    created_at: SystemClock.now(),
                })?,
                json,
            )?;
        }
        crate::commands::KnowledgeCommand::ChangeMapAssignment {
            semantic,
            map_node,
            change,
            rationale,
        } => {
            render_value(
                &service.change_map_assignment(&ChangeSemanticMapAssignmentCommand {
                    semantic_id: semantic,
                    map_node_id: map_node,
                    change: change.into(),
                    rationale,
                    author_kind: "first_party".to_owned(),
                    author: "user".to_owned(),
                    created_at: SystemClock.now(),
                })?,
                json,
            )?;
        }
        crate::commands::KnowledgeCommand::TagMapNode {
            map_node,
            tag,
            change,
            rationale,
        } => {
            render_value(
                &service.change_map_tag(&ChangeMapNodeTagCommand {
                    map_node_id: map_node,
                    tag,
                    change: change.into(),
                    rationale,
                    author_kind: "first_party".to_owned(),
                    author: "user".to_owned(),
                    created_at: SystemClock.now(),
                })?,
                json,
            )?;
        }
        crate::commands::KnowledgeCommand::ShowMapNode { map_node } => {
            render_value(&service.show_map_node(&map_node)?, json)?;
        }
        crate::commands::KnowledgeCommand::BuildDensePreview { semantic } => {
            render_value(&preview_service.build(&semantic)?, json)?;
        }
        crate::commands::KnowledgeCommand::VerifyDensePreview { semantic } => {
            render_value(&preview_service.verify(&semantic)?, json)?;
        }
        crate::commands::KnowledgeCommand::DeleteDensePreview { semantic } => {
            render_value(&preview_service.delete(&semantic)?, json)?;
        }
        crate::commands::KnowledgeCommand::Score {
            semantic,
            map_node,
            interest,
            strategy,
            consensus,
            rationale,
        } => {
            let (target_kind, target_id) = match (semantic, map_node) {
                (Some(semantic), None) => (RelevanceTargetKind::Semantic, semantic),
                (None, Some(map_node)) => (RelevanceTargetKind::MapNode, map_node),
                _ => {
                    return Err(ApplicationError::Integrity(
                        "score requires exactly one --semantic or --map-node".to_owned(),
                    )
                    .into());
                }
            };
            render_value(
                &service.record_score(&RecordRelevanceScoreCommand {
                    target_kind,
                    target_id,
                    components: RelevanceComponents {
                        interest,
                        strategy,
                        consensus,
                        rationale,
                    },
                    author_kind: "first_party".to_owned(),
                    author: "user".to_owned(),
                    created_at: SystemClock.now(),
                })?,
                json,
            )?;
        }
        crate::commands::KnowledgeCommand::Show { suggestion } => {
            render_value(&service.show_semantic(&suggestion)?, json)?;
        }
        crate::commands::KnowledgeCommand::ReviewSuggestion {
            suggestion,
            decision,
            reason,
            first_party_item,
            first_party_revision,
        } => {
            let command = RecordSuggestionReviewCommand {
                suggestion_id: suggestion,
                decision: decision.into(),
                reason,
                first_party_item_id: first_party_item.map(ItemId::parse).transpose()?,
                first_party_revision_id: first_party_revision.map(RevisionId::parse).transpose()?,
                reviewer: "user".to_owned(),
                created_at: SystemClock.now(),
            };
            render_value(&service.review_suggestion(&command)?, json)?;
        }
        crate::commands::KnowledgeCommand::ListProfiles => {
            render_value(&service.score_profiles()?, json)?;
        }
        crate::commands::KnowledgeCommand::CreateProfile {
            interest,
            strategy,
            consensus,
            rationale,
        } => {
            let ordinal = u32::try_from(service.score_profiles()?.len())? + 1;
            let command = CreateScoreProfileCommand {
                profile: ScoreProfile {
                    profile_id: ScoreProfileId::new().to_string(),
                    ordinal,
                    interest_weight: interest,
                    strategy_weight: strategy,
                    consensus_weight: consensus,
                    rationale,
                    author_kind: "first_party".to_owned(),
                    author: "user".to_owned(),
                    created_at: SystemClock.now(),
                },
            };
            render_value(&service.create_score_profile(&command)?, json)?;
        }
    }
    Ok(())
}

fn execute_process(
    command: ProcessCommand,
    config: &AppConfig,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let repository = open_derived_database(&config.paths(), config.sqlite.busy_timeout_ms)?;
    let jobs = open_job_database(&config.paths(), config.sqlite.busy_timeout_ms)?;
    let raw = open_raw_database(&config.paths(), config.sqlite.busy_timeout_ms)?;
    let assets = FileAssetStore::new(config.paths());
    let service = ProcessService::new(repository, raw, assets, SystemClock)
        .with_runtime(jobs, ProcessProviderRouter::detect());
    match command {
        ProcessCommand::ListPipelines => render_value(&service.list_pipelines()?, json)?,
        ProcessCommand::Register { .. } => {
            let register =
                crate::commands::process::build_register_command(&command).map_err(|error| {
                    Box::new(ApplicationError::Integrity(error)) as Box<dyn std::error::Error>
                })?;
            let outcome = service.register_derivative(register)?;
            if json {
                render_value(&outcome, true)?;
            } else {
                match &outcome.derivative_id {
                    Some(derivative_id) => println!("{} {}", outcome.run_id, derivative_id),
                    None => println!("{}", outcome.run_id),
                }
            }
        }
        ProcessCommand::RegisterFailure { .. } => {
            let failure =
                crate::commands::process::build_failure_command(&command).map_err(|error| {
                    Box::new(ApplicationError::Integrity(error)) as Box<dyn std::error::Error>
                })?;
            let outcome = service.register_failure(failure)?;
            if json {
                render_value(&outcome, true)?;
            } else {
                println!("{}", outcome.run_id);
            }
        }
        ProcessCommand::ShowRun { run } => {
            let run_id = RunId::parse(&run).map_err(|error| {
                Box::new(ApplicationError::Domain(error)) as Box<dyn std::error::Error>
            })?;
            render_value(&service.show_run(&run_id)?, json)?;
        }
        ProcessCommand::ListRuns { revision } => {
            let revision_id = RevisionId::parse(&revision).map_err(|error| {
                Box::new(ApplicationError::Domain(error)) as Box<dyn std::error::Error>
            })?;
            render_value(&service.list_runs_for_revision(&revision_id)?, json)?;
        }
        ProcessCommand::DeleteResult { run, reason } => {
            let run_id = RunId::parse(&run).map_err(|error| {
                Box::new(ApplicationError::Domain(error)) as Box<dyn std::error::Error>
            })?;
            render_value(&service.delete_result(&run_id, &reason)?, json)?;
        }
        ProcessCommand::Enqueue { pipeline, revision } => {
            let outcome = service.enqueue(babata_application::EnqueueProcessCommand {
                pipeline_id: PipelineId::new(pipeline),
                revision_id: RevisionId::parse(&revision).map_err(|error| {
                    Box::new(ApplicationError::Domain(error)) as Box<dyn std::error::Error>
                })?,
            })?;
            render_value(&outcome, json)?;
        }
        ProcessCommand::RunOnce => render_value(&service.run_once()?, json)?,
        ProcessCommand::Status { job } => {
            let job_id = parse_job_id(&job)?;
            render_value(&service.status(&job_id)?, json)?;
        }
        ProcessCommand::Retry { job } => {
            let job_id = parse_job_id(&job)?;
            render_value(&service.retry(&job_id)?, json)?;
        }
        ProcessCommand::Cancel { job } => {
            let job_id = parse_job_id(&job)?;
            render_value(&service.cancel(&job_id)?, json)?;
        }
    }
    Ok(())
}

fn parse_job_id(value: &str) -> Result<babata_domain::JobId, Box<dyn std::error::Error>> {
    babata_domain::JobId::parse(value)
        .map_err(|error| Box::new(ApplicationError::Domain(error)) as Box<dyn std::error::Error>)
}

fn unavailable(capability: &str, phase: &str) -> Box<dyn std::error::Error> {
    Box::new(ApplicationError::capability_unavailable(capability, phase))
}
