use babata_domain::SourceRouteDescriptor;

#[derive(Debug, Clone, Default)]
pub struct XiaohongshuConfig {
    pub enabled: bool,
}

pub fn descriptor() -> SourceRouteDescriptor {
    super::super::descriptor("source.xiaohongshu", "xiaohongshu", "P7")
}
