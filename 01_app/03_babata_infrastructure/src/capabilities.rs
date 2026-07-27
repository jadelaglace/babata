use babata_application::{ApplicationError, ports::CapabilityRegistryPort};
use babata_domain::{CapabilityDescriptor, CapabilityId, CapabilityStatus};

#[derive(Debug, Clone)]
pub struct StaticCapabilityRegistry {
    descriptors: Vec<CapabilityDescriptor>,
}

impl Default for StaticCapabilityRegistry {
    fn default() -> Self {
        Self {
            descriptors: all_descriptors(),
        }
    }
}

impl CapabilityRegistryPort for StaticCapabilityRegistry {
    fn list(&self) -> Result<Vec<CapabilityDescriptor>, ApplicationError> {
        Ok(self.descriptors.clone())
    }

    fn get(&self, id: &CapabilityId) -> Result<Option<CapabilityDescriptor>, ApplicationError> {
        Ok(self
            .descriptors
            .iter()
            .find(|descriptor| descriptor.id == *id)
            .cloned())
    }
}

pub fn all_descriptors() -> Vec<CapabilityDescriptor> {
    let mut descriptors = vec![
        CapabilityDescriptor::unavailable("capture.candidate", "P4"),
        disabled_pending_evidence("source.feishu"),
        disabled_pending_evidence("source.kimi"),
        disabled_pending_evidence("source.browser_pages"),
        disabled_pending_evidence("source.browser_bookmarks"),
        disabled_pending_evidence("source.wechat_articles"),
        CapabilityDescriptor::enabled("collector", "P4"),
        CapabilityDescriptor {
            id: CapabilityId::new("processing"),
            status: CapabilityStatus::Enabled,
            activation_phase: "P5".to_owned(),
            reason: Some(
                "C1 register/list/show and the runtime process queue are enabled; individual providers report their live availability"
                    .to_owned(),
            ),
        },
        CapabilityDescriptor {
            id: CapabilityId::new("knowledge.review"),
            status: CapabilityStatus::Enabled,
            activation_phase: "P6.1".to_owned(),
            reason: Some(
                "C0/C1 review preparation and active evidence hash validation are enabled"
                    .to_owned(),
            ),
        },
        CapabilityDescriptor {
            id: CapabilityId::new("knowledge.semantic_core"),
            status: CapabilityStatus::Enabled,
            activation_phase: "P6.1".to_owned(),
            reason: Some(
                "Machine C1 candidates and first-party Log/Insight records enter the same validated semantic core"
                    .to_owned(),
            ),
        },
        CapabilityDescriptor {
            id: CapabilityId::new("knowledge.map_evolution"),
            status: CapabilityStatus::Enabled,
            activation_phase: "P6.1".to_owned(),
            reason: Some(
                "Disciplines, branches, parents, assignments, tags and map-node scores have append-only history while the baseline foundations are locked"
                    .to_owned(),
            ),
        },
        CapabilityDescriptor {
            id: CapabilityId::new("knowledge.dense_preview"),
            status: CapabilityStatus::Enabled,
            activation_phase: "P6.1".to_owned(),
            reason: Some(
                "High-density core text can build, verify, delete and rebuild a controlled C2 Markdown preview"
                    .to_owned(),
            ),
        },
        CapabilityDescriptor::unavailable("knowledge", "P6.1"),
        CapabilityDescriptor {
            id: CapabilityId::new("explore"),
            status: CapabilityStatus::Enabled,
            activation_phase: "P6.2".to_owned(),
            reason: Some(
                "Rebuildable C0/C1 projection, structured search, score ranking, relation navigation and explainable surfacing are enabled"
                    .to_owned(),
            ),
        },
        CapabilityDescriptor {
            id: CapabilityId::new("sublibraries"),
            status: CapabilityStatus::Enabled,
            activation_phase: "P6.3".to_owned(),
            reason: Some(
                "Versioned first-party definitions and disposable materializations are enabled"
                    .to_owned(),
            ),
        },
        CapabilityDescriptor::unavailable("views", "P6"),
        CapabilityDescriptor {
            id: CapabilityId::new("outputs"),
            status: CapabilityStatus::Enabled,
            activation_phase: "P6.3".to_owned(),
            reason: Some(
                "Traceable human-readable Markdown and structured JSON outputs are enabled"
                    .to_owned(),
            ),
        },
        CapabilityDescriptor::unavailable("outputs.web", "unplanned"),
        CapabilityDescriptor::unavailable("outputs.obsidian", "unplanned"),
        CapabilityDescriptor {
            id: CapabilityId::new("ops.backup"),
            status: CapabilityStatus::Enabled,
            activation_phase: "P8.1".to_owned(),
            reason: Some(
                "Consistent SQLite snapshots, encrypted incremental restic backup, and isolated restore verification are enabled"
                    .to_owned(),
            ),
        },
    ];
    descriptors.extend(p7_source_descriptors());
    descriptors.extend(crate::processing::registry::processing_descriptors());
    descriptors.push(
        crate::processing::semantic_digest::BailianSemanticDigestProvider::detect().describe(),
    );
    descriptors
}

fn p7_source_descriptors() -> Vec<CapabilityDescriptor> {
    vec![
        enabled_p7_source(
            "source.onenote",
            "One explicitly authorised official PDF/MHT pair can be structurally verified, collected as complementary exports, and recollected through the core",
        ),
        enabled_p7_source(
            "source.evernote",
            "One explicitly authorised official .notes export can be authenticated, decrypted, collected with resources, and recollected through the core",
        ),
        enabled_p7_source(
            "source.doubao",
            "Explicit batches of 1-20 conversations can be discovered from the signed-in history, collected through OpenCLI into C0, and recollected per item without incomplete pagination or transient browser fields becoming C0",
        ),
    ]
}

fn disabled_pending_evidence(id: &str) -> CapabilityDescriptor {
    CapabilityDescriptor {
        id: CapabilityId::new(id),
        status: CapabilityStatus::Disabled,
        activation_phase: "P4".to_owned(),
        reason: Some("awaiting authorised contextual collection evidence".to_owned()),
    }
}

fn enabled_p7_source(id: &str, reason: &str) -> CapabilityDescriptor {
    CapabilityDescriptor {
        id: CapabilityId::new(id),
        status: CapabilityStatus::Enabled,
        activation_phase: "P7".to_owned(),
        reason: Some(reason.to_owned()),
    }
}
