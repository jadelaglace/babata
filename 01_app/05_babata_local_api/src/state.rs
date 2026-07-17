use std::net::{IpAddr, Ipv4Addr};

#[derive(Debug, Clone)]
pub struct ApiState {
    pub enabled: bool,
    pub bind_address: IpAddr,
}

impl ApiState {
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            bind_address: IpAddr::V4(Ipv4Addr::LOCALHOST),
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn default_state_is_disabled_and_loopback_only() {
        let state = super::ApiState::disabled();
        assert!(!state.enabled);
        assert!(state.bind_address.is_loopback());
    }
}
