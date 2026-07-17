use crate::{routes, state::ApiState};

#[derive(Debug, Clone)]
pub struct ApiDescriptor {
    pub state: ApiState,
    pub endpoints: Vec<routes::Endpoint>,
}

pub fn build() -> ApiDescriptor {
    ApiDescriptor {
        state: ApiState::disabled(),
        endpoints: routes::all(),
    }
}
