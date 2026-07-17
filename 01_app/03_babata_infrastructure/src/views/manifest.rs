use serde::{Deserialize, Serialize};

use babata_domain::{OutputId, OutputManifestRef};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputManifest {
    pub output_id: OutputId,
    pub reference: OutputManifestRef,
    pub input_versions: Vec<String>,
    pub builder_version: String,
}
