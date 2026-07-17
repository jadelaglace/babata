use babata_domain::SourceRouteDescriptor;

#[derive(Debug, Clone, Default)]
pub struct OneNoteConfig {
    pub enabled: bool,
}

pub fn descriptor() -> SourceRouteDescriptor {
    super::super::descriptor("source.onenote", "onenote", "P7")
}
