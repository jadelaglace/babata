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
        CapabilityDescriptor::unavailable("knowledge", "P6"),
        CapabilityDescriptor::unavailable("explore", "P6"),
        CapabilityDescriptor::unavailable("sublibraries", "P6"),
        CapabilityDescriptor::unavailable("views", "P6"),
        CapabilityDescriptor::unavailable("outputs", "P6"),
        CapabilityDescriptor::unavailable("ops.backup", "P8"),
    ];
    descriptors.extend(crate::processing::registry::processing_descriptors());
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
