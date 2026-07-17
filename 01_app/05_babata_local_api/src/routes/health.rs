use super::Endpoint;

pub const ENDPOINTS: [Endpoint; 1] = [Endpoint {
    method: "GET",
    path: "/v1/health",
    capability: "api.health",
    activation_phase: "P2",
}];
