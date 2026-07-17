use babata_domain::SourceRouteDescriptor;

#[derive(Debug, Clone, Default)]
pub struct ZhihuConfig {
    pub enabled: bool,
}

pub fn descriptor() -> SourceRouteDescriptor {
    super::super::descriptor("source.zhihu", "zhihu", "P7")
}
