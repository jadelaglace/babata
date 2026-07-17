use babata_domain::SourceRouteDescriptor;

#[derive(Debug, Clone, Default)]
pub struct FirstPartyConfig {
    pub enabled: bool,
}

pub fn descriptor() -> SourceRouteDescriptor {
    super::super::descriptor("source.first_party", "first_party", "P3")
}
