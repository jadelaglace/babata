use babata_domain::CapabilityDescriptor;

pub fn descriptors() -> Vec<CapabilityDescriptor> {
    vec![
        CapabilityDescriptor::unavailable("processing.local_extract", "P5"),
        CapabilityDescriptor::unavailable("processing.bailian_cli", "P5"),
        CapabilityDescriptor::unavailable("processing.bailian_api", "P5+"),
    ]
}
