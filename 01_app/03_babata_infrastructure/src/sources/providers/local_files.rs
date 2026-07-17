use babata_domain::SourceRouteDescriptor;

#[derive(Debug, Clone, Default)]
pub struct LocalFilesConfig {
    pub enabled: bool,
}

pub fn descriptor() -> SourceRouteDescriptor {
    super::super::descriptor("source.local_files", "local_files", "P3")
}
