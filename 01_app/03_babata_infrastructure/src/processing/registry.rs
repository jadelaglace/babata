use babata_domain::CapabilityDescriptor;

pub fn processing_descriptors() -> Vec<CapabilityDescriptor> {
    vec![
        CapabilityDescriptor::unavailable("processing.local_extract", "P5"),
        CapabilityDescriptor::unavailable("processing.bailian_cli", "P5"),
        CapabilityDescriptor::unavailable("processing.bailian_api", "P5+"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use babata_domain::CapabilityStatus;

    #[test]
    fn providers_stay_unavailable_until_the_job_queue_can_call_them() {
        let descriptors = processing_descriptors();
        assert!(
            descriptors
                .iter()
                .all(|descriptor| descriptor.status == CapabilityStatus::Unavailable)
        );
    }
}
