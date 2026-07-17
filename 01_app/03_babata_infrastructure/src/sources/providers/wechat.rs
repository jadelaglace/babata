use babata_domain::SourceRouteDescriptor;

#[derive(Debug, Clone, Default)]
pub struct WechatConfig {
    pub enabled: bool,
}

pub fn descriptor() -> SourceRouteDescriptor {
    super::super::descriptor("source.wechat", "wechat", "P7")
}
