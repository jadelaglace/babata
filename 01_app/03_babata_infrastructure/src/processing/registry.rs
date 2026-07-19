use babata_domain::CapabilityDescriptor;

pub fn processing_descriptors() -> Vec<CapabilityDescriptor> {
    vec![
        CapabilityDescriptor::enabled("processing.local_extract", "P5"),
        CapabilityDescriptor {
            id: babata_domain::CapabilityId::new("processing.bailian_cli"),
            status: babata_domain::CapabilityStatus::Enabled,
            activation_phase: "P5".to_owned(),
            reason: Some("register path enabled; live provider job queue still P5+".to_owned()),
        },
        CapabilityDescriptor::unavailable("processing.bailian_api", "P5+"),
    ]
}
