use babata_domain::SourceRouteDescriptor;

#[derive(Debug, Clone, Default)]
pub struct BilibiliConfig {
    pub enabled: bool,
}

pub fn descriptor() -> SourceRouteDescriptor {
    super::super::descriptor("source.bilibili", "bilibili", "P7")
}
