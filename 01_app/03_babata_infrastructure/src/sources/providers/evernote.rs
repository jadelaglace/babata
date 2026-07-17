use babata_domain::SourceRouteDescriptor;

#[derive(Debug, Clone, Default)]
pub struct EvernoteConfig {
    pub enabled: bool,
}

pub fn descriptor() -> SourceRouteDescriptor {
    super::super::descriptor("source.evernote", "evernote", "P7")
}
