use babata_domain::SourceRouteDescriptor;

#[derive(Debug, Clone, Default)]
pub struct DouyinConfig {
    pub enabled: bool,
}

pub fn descriptor() -> SourceRouteDescriptor {
    super::super::descriptor("source.douyin", "douyin", "P7")
}
