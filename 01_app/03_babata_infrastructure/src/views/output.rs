use std::{
    collections::{BTreeMap, BTreeSet},
    fs, io,
    path::{Path, PathBuf},
};

use babata_application::{ApplicationError, ports::OutputBuilderPort};
use babata_domain::{
    OUTPUT_MANIFEST_SCHEMA_VERSION, OutputBuild, OutputDocument, OutputId, OutputKind,
    OutputManifestRef, OutputScope, OutputScoreProfileRef, OutputState, OutputVerification, Sha256,
    UtcTimestamp,
};
use serde::{Deserialize, Serialize};

use crate::paths::{DataPaths, ensure_layout};

#[derive(Debug, Clone)]
pub struct OutputViewStore {
    paths: DataPaths,
}

impl OutputViewStore {
    pub fn new(paths: DataPaths) -> Self {
        Self { paths }
    }

    fn root(&self) -> PathBuf {
        self.paths.root().join("03_views/outputs")
    }

    fn canonical_root(&self) -> Result<PathBuf, ApplicationError> {
        let canonical_data_root = self.paths.root().canonicalize().map_err(io_error)?;
        let views = self.paths.root().join("03_views");
        let canonical_views = views.canonicalize().map_err(io_error)?;
        let canonical_root = self.root().canonicalize().map_err(io_error)?;
        if canonical_views.parent() != Some(canonical_data_root.as_path())
            || canonical_root.parent() != Some(canonical_views.as_path())
        {
            return Err(ApplicationError::Integrity(
                "output C2 root escaped BABATA_DATA_HOME".to_owned(),
            ));
        }
        Ok(canonical_root)
    }

    fn directory(&self, output_id: &OutputId) -> PathBuf {
        self.root().join(output_id.as_str())
    }

    fn ensure_safe_existing_directory(
        &self,
        output_id: &OutputId,
    ) -> Result<PathBuf, ApplicationError> {
        let directory = self.directory(output_id);
        if !directory.exists() {
            return Err(ApplicationError::NotFound(output_id.to_string()));
        }
        let canonical_root = self.canonical_root()?;
        let canonical_directory = directory.canonicalize().map_err(io_error)?;
        if canonical_directory.parent() != Some(canonical_root.as_path()) {
            return Err(ApplicationError::Integrity(
                "output escaped its controlled C2 root".to_owned(),
            ));
        }
        Ok(directory)
    }

    fn read_manifest(&self, output_id: &OutputId) -> Result<OutputManifest, ApplicationError> {
        let directory = self.ensure_safe_existing_directory(output_id)?;
        let bytes = fs::read(directory.join("manifest.json")).map_err(io_error)?;
        let manifest: OutputManifest = serde_json::from_slice(&bytes)
            .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
        validate_manifest(&manifest, output_id)?;
        read_history(&directory, &manifest)?;
        Ok(manifest)
    }

    fn build_internal(
        &self,
        document: &OutputDocument,
        rebuild: bool,
    ) -> Result<OutputBuild, ApplicationError> {
        ensure_layout(&self.paths).map_err(io_error)?;
        fs::create_dir_all(self.root()).map_err(io_error)?;
        self.canonical_root()?;
        let directory = self.directory(&document.id);
        let previous = if rebuild {
            Some(self.read_manifest(&document.id)?)
        } else {
            if directory.exists() {
                return Err(ApplicationError::Conflict(format!(
                    "output {} already exists",
                    document.id
                )));
            }
            None
        };
        if directory.exists() {
            self.ensure_safe_existing_directory(&document.id)?;
        } else {
            fs::create_dir(&directory).map_err(io_error)?;
        }
        let artifact_name = artifact_name(document.kind)?;
        let artifact = render(document)?;
        let output_sha256 = Sha256::of_bytes(&artifact);
        let generation = previous.as_ref().map_or(Ok(1), |manifest| {
            manifest
                .generation
                .checked_add(1)
                .ok_or_else(|| ApplicationError::Integrity("output generation overflow".to_owned()))
        })?;
        let mut differences = previous.as_ref().map_or_else(Vec::new, |manifest| {
            compare_inputs(&manifest.inputs, document)
        });
        if previous
            .as_ref()
            .is_some_and(|manifest| manifest.output_sha256 != output_sha256.as_str())
        {
            differences.push("artifact hash changed".to_owned());
        }
        let previous_bytes = previous
            .as_ref()
            .map(serde_json::to_vec_pretty)
            .transpose()
            .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
        if let (Some(previous), Some(previous_bytes)) = (&previous, &previous_bytes) {
            let history = directory.join("history");
            fs::create_dir_all(&history).map_err(io_error)?;
            replace_file(
                history.join(format!("manifest-v{}.json", previous.generation)),
                previous_bytes,
            )?;
        }
        for stale in ["output.md", "output.json"] {
            let path = directory.join(stale);
            if stale != artifact_name && path.exists() {
                fs::remove_file(path).map_err(io_error)?;
            }
        }
        replace_file(directory.join(artifact_name), &artifact)?;
        let relative_root = format!("03_views/outputs/{}", document.id);
        let manifest = OutputManifest {
            schema_version: OUTPUT_MANIFEST_SCHEMA_VERSION.to_owned(),
            output_id: document.id.clone(),
            kind: document.kind,
            scope: document.scope.clone(),
            state: OutputState::Succeeded,
            generation,
            builder_version: document.builder_version.clone(),
            template_version: document.template_version.clone(),
            score_profiles: document.score_profiles.clone(),
            generated_at: document.generated_at.clone(),
            artifact_path: format!("{relative_root}/{artifact_name}"),
            artifact_file: artifact_name.to_owned(),
            output_sha256: output_sha256.to_string(),
            manifest_path: format!("{relative_root}/manifest.json"),
            inputs: document
                .records
                .iter()
                .map(OutputInputRef::from_document)
                .collect(),
            limitations: document.limitations.clone(),
            differences,
            previous_manifest_sha256: previous_bytes
                .as_ref()
                .map(|bytes| Sha256::of_bytes(bytes).to_string()),
        };
        replace_file(
            directory.join("manifest.json"),
            &serde_json::to_vec_pretty(&manifest)
                .map_err(|error| ApplicationError::Integrity(error.to_string()))?,
        )?;
        manifest.into_build()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OutputManifest {
    schema_version: String,
    output_id: OutputId,
    kind: OutputKind,
    scope: OutputScope,
    state: OutputState,
    generation: u32,
    builder_version: String,
    template_version: String,
    #[serde(default)]
    score_profiles: Vec<OutputScoreProfileRef>,
    generated_at: UtcTimestamp,
    artifact_path: String,
    artifact_file: String,
    output_sha256: String,
    manifest_path: String,
    inputs: Vec<OutputInputRef>,
    limitations: Vec<String>,
    differences: Vec<String>,
    previous_manifest_sha256: Option<String>,
}

impl OutputManifest {
    fn into_build(self) -> Result<OutputBuild, ApplicationError> {
        Ok(OutputBuild {
            id: self.output_id,
            kind: self.kind,
            scope: self.scope,
            state: self.state,
            generation: self.generation,
            builder_version: self.builder_version,
            template_version: self.template_version,
            artifact_path: self.artifact_path,
            output_sha256: Sha256::parse(self.output_sha256)?,
            manifest: OutputManifestRef {
                relative_path: self.manifest_path,
            },
            differences: self.differences,
            generated_at: self.generated_at,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct OutputInputRef {
    record_id: String,
    item_id: Option<String>,
    revision_id: Option<String>,
    semantic_id: Option<String>,
    input_sha256: String,
    source_id: String,
    source_locator: Option<String>,
    source_native_id: Option<String>,
    origin_kind: String,
    review_state: Option<String>,
    human_judgment: bool,
    confirmed_fact: bool,
    #[serde(default)]
    score_profile: Option<OutputScoreProfileRef>,
    limitations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LegacyOutputManifest {
    schema_version: String,
    output_id: OutputId,
    kind: OutputKind,
    scope: OutputScope,
    state: OutputState,
    generation: u32,
    builder_version: String,
    template_version: String,
    profile_id: String,
    generated_at: UtcTimestamp,
    artifact_path: String,
    artifact_file: String,
    output_sha256: String,
    manifest_path: String,
    inputs: Vec<LegacyOutputInputRef>,
    limitations: Vec<String>,
    differences: Vec<String>,
    previous_manifest_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LegacyOutputInputRef {
    record_id: String,
    item_id: Option<String>,
    revision_id: Option<String>,
    semantic_id: Option<String>,
    input_sha256: String,
    source_id: String,
    source_locator: Option<String>,
    source_native_id: Option<String>,
    origin_kind: String,
    review_state: Option<String>,
    human_judgment: bool,
    confirmed_fact: bool,
    limitations: Vec<String>,
}

impl OutputInputRef {
    fn from_document(record: &babata_domain::OutputInputRecord) -> Self {
        let summary = &record.detail.record;
        Self {
            record_id: summary.record_id.clone(),
            item_id: summary.item_id.as_ref().map(ToString::to_string),
            revision_id: summary.revision_id.as_ref().map(ToString::to_string),
            semantic_id: summary.semantic_id.clone(),
            input_sha256: record.input_sha256.to_string(),
            source_id: summary.source_id.to_string(),
            source_locator: summary.source_locator.clone(),
            source_native_id: summary.source_native_id.clone(),
            origin_kind: summary.origin_kind.clone(),
            review_state: summary.review_state.clone(),
            human_judgment: summary.judgment.human_judgment,
            confirmed_fact: summary.judgment.confirmed_fact,
            score_profile: summary.score.as_ref().map(|score| OutputScoreProfileRef {
                profile_id: score.profile_id.clone(),
                profile_ordinal: score.profile_ordinal,
                interest_weight: score.interest_weight,
                strategy_weight: score.strategy_weight,
                consensus_weight: score.consensus_weight,
            }),
            limitations: summary.limitations.clone(),
        }
    }
}

impl OutputBuilderPort for OutputViewStore {
    fn supported_kinds(&self) -> Vec<OutputKind> {
        vec![OutputKind::HumanReadable, OutputKind::Structured]
    }

    fn build(&self, document: &OutputDocument) -> Result<OutputBuild, ApplicationError> {
        self.build_internal(document, false)
    }

    fn rebuild(&self, document: &OutputDocument) -> Result<OutputBuild, ApplicationError> {
        self.build_internal(document, true)
    }

    fn status(&self, output_id: &OutputId) -> Result<OutputBuild, ApplicationError> {
        self.read_manifest(output_id)?.into_build()
    }

    fn verify(&self, document: &OutputDocument) -> Result<OutputVerification, ApplicationError> {
        let output_id = &document.id;
        let manifest = self.read_manifest(output_id)?;
        let expected_inputs = document
            .records
            .iter()
            .map(OutputInputRef::from_document)
            .collect::<Vec<_>>();
        let expected = Sha256::of_bytes(&render(document)?);
        let history = read_history(&self.ensure_safe_existing_directory(output_id)?, &manifest)?;
        let mut expected_differences = history.first().map_or_else(Vec::new, |previous| {
            compare_inputs(&previous.inputs, document)
        });
        if history
            .first()
            .is_some_and(|previous| previous.output_sha256 != expected.as_str())
        {
            expected_differences.push("artifact hash changed".to_owned());
        }
        let manifest_matches_inputs = manifest.kind == document.kind
            && manifest.scope == document.scope
            && manifest.builder_version == document.builder_version
            && manifest.template_version == document.template_version
            && manifest.score_profiles == document.score_profiles
            && manifest.generated_at == document.generated_at
            && manifest.inputs == expected_inputs
            && manifest.limitations == document.limitations
            && manifest.differences == expected_differences
            && manifest.output_sha256 == expected.as_str();
        let artifact = self
            .ensure_safe_existing_directory(output_id)?
            .join(&manifest.artifact_file);
        let actual = if artifact.exists() {
            Some(Sha256::of_bytes(&fs::read(artifact).map_err(io_error)?))
        } else {
            None
        };
        let valid = manifest_matches_inputs
            && actual.as_ref() == Some(&expected)
            && manifest.state == OutputState::Succeeded;
        Ok(OutputVerification {
            output_id: output_id.clone(),
            valid,
            expected_sha256: expected,
            actual_sha256: actual,
            detail: if valid {
                "artifact and manifest match authoritative input references".to_owned()
            } else {
                "artifact or manifest is missing, deleted, modified, or differs from authoritative input references".to_owned()
            },
        })
    }

    fn delete(&self, output_id: &OutputId) -> Result<OutputBuild, ApplicationError> {
        let mut manifest = self.read_manifest(output_id)?;
        let directory = self.ensure_safe_existing_directory(output_id)?;
        let artifact = directory.join(&manifest.artifact_file);
        if artifact.exists() {
            fs::remove_file(artifact).map_err(io_error)?;
        }
        manifest.state = OutputState::Deleted;
        replace_file(
            directory.join("manifest.json"),
            &serde_json::to_vec_pretty(&manifest)
                .map_err(|error| ApplicationError::Integrity(error.to_string()))?,
        )?;
        manifest.into_build()
    }
}

fn validate_manifest(
    manifest: &OutputManifest,
    output_id: &OutputId,
) -> Result<(), ApplicationError> {
    if manifest.schema_version != OUTPUT_MANIFEST_SCHEMA_VERSION || manifest.output_id != *output_id
    {
        return Err(ApplicationError::Integrity(
            "output manifest identity is invalid".to_owned(),
        ));
    }
    let artifact_name = artifact_name(manifest.kind)?;
    let relative_root = format!("03_views/outputs/{output_id}");
    if manifest.generation == 0
        || manifest.artifact_file != artifact_name
        || manifest.artifact_path != format!("{relative_root}/{artifact_name}")
        || manifest.manifest_path != format!("{relative_root}/manifest.json")
    {
        return Err(ApplicationError::Integrity(
            "output manifest paths or generation are invalid".to_owned(),
        ));
    }
    Sha256::parse(&manifest.output_sha256)?;
    Ok(())
}

fn read_history(
    directory: &Path,
    manifest: &OutputManifest,
) -> Result<Vec<OutputManifest>, ApplicationError> {
    let mut current = manifest.clone();
    let mut history = Vec::new();
    while current.generation > 1 {
        let prior_generation = current.generation - 1;
        let expected_hash = current.previous_manifest_sha256.as_deref().ok_or_else(|| {
            ApplicationError::Integrity("output manifest history link is missing".to_owned())
        })?;
        let expected_hash = Sha256::parse(expected_hash)?;
        let bytes = fs::read(
            directory
                .join("history")
                .join(format!("manifest-v{prior_generation}.json")),
        )
        .map_err(io_error)?;
        let previous: OutputManifest = serde_json::from_slice(&bytes)
            .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
        if !history_hash_matches(&expected_hash, &bytes, &previous)? {
            return Err(ApplicationError::Integrity(
                "output manifest history hash is invalid".to_owned(),
            ));
        }
        validate_manifest(&previous, &manifest.output_id)?;
        if previous.generation != prior_generation {
            return Err(ApplicationError::Integrity(
                "output manifest history generation is invalid".to_owned(),
            ));
        }
        history.push(previous.clone());
        current = previous;
    }
    if current.previous_manifest_sha256.is_some() {
        return Err(ApplicationError::Integrity(
            "initial output manifest has an invalid history link".to_owned(),
        ));
    }
    Ok(history)
}

fn history_hash_matches(
    expected: &Sha256,
    stored_bytes: &[u8],
    manifest: &OutputManifest,
) -> Result<bool, ApplicationError> {
    if Sha256::of_bytes(stored_bytes) == *expected {
        return Ok(true);
    }
    let legacy_canonical = serde_json::to_vec(manifest)
        .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
    if Sha256::of_bytes(&legacy_canonical) == *expected {
        return Ok(true);
    }
    let Ok(legacy) = serde_json::from_slice::<LegacyOutputManifest>(stored_bytes) else {
        return Ok(false);
    };
    let legacy_canonical = serde_json::to_vec(&legacy)
        .map_err(|error| ApplicationError::Integrity(error.to_string()))?;
    Ok(Sha256::of_bytes(&legacy_canonical) == *expected)
}

fn render(document: &OutputDocument) -> Result<Vec<u8>, ApplicationError> {
    match document.kind {
        OutputKind::Structured => serde_json::to_vec_pretty(&serde_json::json!({
            "schema_version": "babata.structured-output/v1",
            "output_id": document.id,
            "scope": document.scope,
            "generated_at": document.generated_at,
            "builder_version": document.builder_version,
            "template_version": document.template_version,
            "score_profiles": document.score_profiles,
            "limitations": document.limitations,
            "records": document.records,
        }))
        .map_err(|error| ApplicationError::Integrity(error.to_string())),
        OutputKind::HumanReadable => Ok(render_markdown(document).into_bytes()),
        OutputKind::Web | OutputKind::Obsidian => Err(ApplicationError::capability_unavailable(
            format!("outputs.{:?}", document.kind).to_lowercase(),
            "unplanned",
        )),
    }
}

fn artifact_name(kind: OutputKind) -> Result<&'static str, ApplicationError> {
    match kind {
        OutputKind::HumanReadable => Ok("output.md"),
        OutputKind::Structured => Ok("output.json"),
        OutputKind::Web | OutputKind::Obsidian => Err(ApplicationError::capability_unavailable(
            format!("outputs.{kind:?}").to_lowercase(),
            "unplanned",
        )),
    }
}

fn render_markdown(document: &OutputDocument) -> String {
    use std::fmt::Write;

    let mut output = String::new();
    let _ = writeln!(output, "# {}\n", document.scope.description);
    let _ = writeln!(output, "- Output: `{}`", document.id);
    let _ = writeln!(output, "- Generated: `{}`", document.generated_at.as_str());
    let _ = writeln!(output, "- Builder: `{}`", document.builder_version);
    let _ = writeln!(output, "- Template: `{}`", document.template_version);
    for profile in &document.score_profiles {
        let _ = writeln!(
            output,
            "- Score profile: `{}` v{} ({}/{}/{})",
            profile.profile_id,
            profile.profile_ordinal,
            profile.interest_weight,
            profile.strategy_weight,
            profile.consensus_weight
        );
    }
    output.push('\n');
    for input in &document.records {
        let record = &input.detail.record;
        let _ = writeln!(output, "## {}\n", record.title);
        let _ = writeln!(output, "- Record: `{}`", record.record_id);
        if let Some(item_id) = &record.item_id {
            let _ = writeln!(output, "- Item: `{item_id}`");
        }
        if let Some(revision_id) = &record.revision_id {
            let _ = writeln!(output, "- Revision: `{revision_id}`");
        }
        if let Some(semantic_id) = &record.semantic_id {
            let _ = writeln!(output, "- Semantic entry: `{semantic_id}`");
        }
        let _ = writeln!(output, "- Input hash: `{}`", input.input_sha256);
        let _ = writeln!(
            output,
            "- Source: `{}` / `{}`",
            record.provider, record.source_id
        );
        if let Some(locator) = &record.source_locator {
            let _ = writeln!(output, "- Source locator: {locator}");
        }
        let _ = writeln!(
            output,
            "- Identity: origin=`{}`, review=`{}`, human_judgment=`{}`, confirmed_fact=`{}`",
            record.origin_kind,
            record.review_state.as_deref().unwrap_or("none"),
            record.judgment.human_judgment,
            record.judgment.confirmed_fact
        );
        if let Some(excerpt) = &record.excerpt {
            let _ = writeln!(output, "\n{excerpt}\n");
        }
        if !input.detail.relations.is_empty() {
            output.push_str("Relations:\n\n");
            for relation in &input.detail.relations {
                let _ = writeln!(
                    output,
                    "- `{}` -> `{}`{}",
                    relation.relation_kind,
                    relation.related_entity_id,
                    if relation.broken { " (broken)" } else { "" }
                );
            }
            output.push('\n');
        }
        if !record.limitations.is_empty() {
            output.push_str("Limitations:\n\n");
            for limitation in &record.limitations {
                let _ = writeln!(output, "- {limitation}");
            }
            output.push('\n');
        }
    }
    output
}

fn compare_inputs(previous: &[OutputInputRef], document: &OutputDocument) -> Vec<String> {
    let old = previous
        .iter()
        .map(|input| (input.record_id.clone(), input.input_sha256.clone()))
        .collect::<BTreeMap<_, _>>();
    let new = document
        .records
        .iter()
        .map(|input| {
            (
                input.detail.record.record_id.clone(),
                input.input_sha256.to_string(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let old_ids = old.keys().cloned().collect::<BTreeSet<_>>();
    let new_ids = new.keys().cloned().collect::<BTreeSet<_>>();
    let mut differences = old_ids
        .difference(&new_ids)
        .map(|record_id| format!("removed input {record_id}"))
        .chain(
            new_ids
                .difference(&old_ids)
                .map(|record_id| format!("added input {record_id}")),
        )
        .collect::<Vec<_>>();
    for record_id in old_ids.intersection(&new_ids) {
        if old.get(record_id) != new.get(record_id) {
            differences.push(format!("changed input {record_id}"));
        }
    }
    differences
}

fn replace_file(path: PathBuf, bytes: &[u8]) -> Result<(), ApplicationError> {
    let temporary = path.with_extension("tmp");
    fs::write(&temporary, bytes).map_err(io_error)?;
    if path.exists() {
        fs::remove_file(&path).map_err(io_error)?;
    }
    fs::rename(temporary, path).map_err(io_error)
}

fn io_error(error: io::Error) -> ApplicationError {
    ApplicationError::Storage(format!("C2 output filesystem {:?} failure", error.kind()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_hash_accepts_saved_bytes_and_the_legacy_compact_contract() {
        let manifest = OutputManifest {
            schema_version: OUTPUT_MANIFEST_SCHEMA_VERSION.to_owned(),
            output_id: OutputId::parse("output_01J00000000000000000000000").unwrap(),
            kind: OutputKind::Structured,
            scope: OutputScope {
                record_ids: vec!["item:item_01J00000000000000000000000".to_owned()],
                sublibrary: None,
                description: "Fixture".to_owned(),
            },
            state: OutputState::Succeeded,
            generation: 1,
            builder_version: "builder/v1".to_owned(),
            template_version: "template/v1".to_owned(),
            score_profiles: Vec::new(),
            generated_at: UtcTimestamp::parse("2026-07-23T00:00:00Z").unwrap(),
            artifact_path: "03_views/outputs/output_01J00000000000000000000000/output.json"
                .to_owned(),
            artifact_file: "output.json".to_owned(),
            output_sha256: "a".repeat(64),
            manifest_path: "03_views/outputs/output_01J00000000000000000000000/manifest.json"
                .to_owned(),
            inputs: Vec::new(),
            limitations: Vec::new(),
            differences: Vec::new(),
            previous_manifest_sha256: None,
        };
        let pretty = serde_json::to_vec_pretty(&manifest).unwrap();
        let compact = serde_json::to_vec(&manifest).unwrap();
        assert!(history_hash_matches(&Sha256::of_bytes(&pretty), &pretty, &manifest).unwrap());
        assert!(history_hash_matches(&Sha256::of_bytes(&compact), &pretty, &manifest).unwrap());
        let legacy = LegacyOutputManifest {
            schema_version: manifest.schema_version.clone(),
            output_id: manifest.output_id.clone(),
            kind: manifest.kind,
            scope: manifest.scope.clone(),
            state: manifest.state,
            generation: manifest.generation,
            builder_version: manifest.builder_version.clone(),
            template_version: manifest.template_version.clone(),
            profile_id: "score_profile_p6_default".to_owned(),
            generated_at: manifest.generated_at.clone(),
            artifact_path: manifest.artifact_path.clone(),
            artifact_file: manifest.artifact_file.clone(),
            output_sha256: manifest.output_sha256.clone(),
            manifest_path: manifest.manifest_path.clone(),
            inputs: Vec::new(),
            limitations: Vec::new(),
            differences: Vec::new(),
            previous_manifest_sha256: None,
        };
        let legacy_pretty = serde_json::to_vec_pretty(&legacy).unwrap();
        let legacy_compact = serde_json::to_vec(&legacy).unwrap();
        let current_view: OutputManifest = serde_json::from_slice(&legacy_pretty).unwrap();
        assert!(
            history_hash_matches(
                &Sha256::of_bytes(&legacy_compact),
                &legacy_pretty,
                &current_view,
            )
            .unwrap()
        );
        assert!(
            !history_hash_matches(&Sha256::of_bytes(b"different"), &pretty, &manifest).unwrap()
        );
    }
}
