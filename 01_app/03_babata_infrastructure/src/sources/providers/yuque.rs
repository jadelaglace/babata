use babata_domain::SourceRouteDescriptor;

#[derive(Debug, Clone, Default)]
pub struct YuqueConfig {
    pub enabled: bool,
}

pub fn descriptor() -> SourceRouteDescriptor {
    super::super::descriptor("source.yuque", "yuque", "P7")
}
