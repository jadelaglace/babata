use super::Endpoint;

pub const ENDPOINTS: [Endpoint; 1] = [Endpoint {
    method: "POST",
    path: "/v1/explore/search",
    capability: "explore",
    activation_phase: "P6",
}];
