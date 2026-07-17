use std::{fs, path::Path};

use babata_application::ApplicationError;
use babata_domain::{CapabilityStatus, SourceRouteDescriptor, SourceRouteId};

#[derive(Debug, Clone, Default)]
pub struct FeishuConfig {
    pub enabled: bool,
}

pub fn descriptor() -> SourceRouteDescriptor {
    SourceRouteDescriptor {
        id: SourceRouteId("source.feishu".to_owned()),
        provider: "feishu".to_owned(),
        status: CapabilityStatus::Disabled,
        activation_phase: "P4".to_owned(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeishuMarkdownExport {
    pub title: String,
    pub raw_text: String,
}

pub fn read_markdown_export(path: &Path) -> Result<FeishuMarkdownExport, ApplicationError> {
    let raw_text = fs::read_to_string(path).map_err(|error| {
        ApplicationError::Asset(format!("unable to read Feishu export: {:?}", error.kind()))
    })?;
    let title = raw_text
        .lines()
        .find_map(|line| line.trim().strip_prefix("# ").map(str::trim))
        .filter(|title| !title.is_empty())
        .map(str::to_owned)
        .or_else(|| {
            path.file_stem()
                .and_then(|value| value.to_str())
                .filter(|title| !title.is_empty())
                .map(str::to_owned)
        })
        .ok_or_else(|| ApplicationError::Asset("Feishu export title is unavailable".to_owned()))?;
    if raw_text.trim().is_empty() {
        return Err(ApplicationError::Asset(
            "Feishu export must contain text".to_owned(),
        ));
    }
    Ok(FeishuMarkdownExport { title, raw_text })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn markdown_export_uses_heading_as_title() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("export.md");
        fs::write(&path, "# A Feishu document\n\nBody").unwrap();
        let export = read_markdown_export(&path).unwrap();
        assert_eq!(export.title, "A Feishu document");
        assert_eq!(export.raw_text, "# A Feishu document\n\nBody");
    }
}
