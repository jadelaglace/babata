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
        CapabilityDescriptor::unavailable("explore", "P6"),
        CapabilityDescriptor::unavailable("sublibraries", "P6"),
        CapabilityDescriptor::unavailable("views", "P6"),
        CapabilityDescriptor::unavailable("outputs", "P6"),
        CapabilityDescriptor::unavailable("ops.backup", "P8"),
    ];
    descriptors.extend(crate::processing::registry::processing_descriptors());
    descriptors.push(
        crate::processing::semantic_digest::BailianSemanticDigestProvider::detect().describe(),
    );
    descriptors
}

fn disabled_pending_evidence(id: &str) -> CapabilityDescriptor {
    CapabilityDescriptor {
        id: CapabilityId::new(id),
        status: CapabilityStatus::Disabled,
        activation_phase: "P4".to_owned(),
        reason: Some("awaiting authorised contextual collection evidence".to_owned()),
    }
}
