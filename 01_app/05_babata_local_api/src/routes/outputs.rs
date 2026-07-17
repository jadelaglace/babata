use super::Endpoint;

pub const ENDPOINTS: [Endpoint; 4] = [
    Endpoint {
        method: "GET",
        path: "/v1/outputs",
        capability: "outputs.list",
        activation_phase: "P6",
    },
    Endpoint {
        method: "POST",
        path: "/v1/outputs/build",
        capability: "outputs.build",
        activation_phase: "P6",
    },
    Endpoint {
        method: "GET",
        path: "/v1/outputs/status",
        capability: "outputs.status",
        activation_phase: "P6",
    },
    Endpoint {
        method: "POST",
        path: "/v1/outputs/verify",
        capability: "outputs.verify",
        activation_phase: "P6",
    },
];
