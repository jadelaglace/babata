use babata_domain::SourceRouteDescriptor;

#[derive(Debug, Clone, Default)]
pub struct ConversationsConfig {
    pub enabled: bool,
}

pub fn descriptor() -> SourceRouteDescriptor {
    super::super::descriptor("source.conversations", "conversations", "P7")
}
