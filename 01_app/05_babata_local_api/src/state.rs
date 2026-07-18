use std::net::{IpAddr, Ipv4Addr};

use crate::ApiError;

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

    pub fn enabled(bind_address: IpAddr) -> Result<Self, ApiError> {
        if !bind_address.is_loopback() {
            return Err(ApiError::InvalidRequest(
                "local API must bind to a loopback address".to_owned(),
            ));
        }
        Ok(Self {
            enabled: true,
            bind_address,
        })
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

    #[test]
    fn enabled_state_rejects_non_loopback_bindings() {
        assert!(super::ApiState::enabled("127.0.0.1".parse().unwrap()).is_ok());
        assert!(super::ApiState::enabled("0.0.0.0".parse().unwrap()).is_err());
    }
}
