use super::Endpoint;

pub const ENDPOINTS: [Endpoint; 3] = [
    Endpoint {
        method: "POST",
        path: "/v1/process/enqueue",
        capability: "processing",
        activation_phase: "P5",
    },
    Endpoint {
        method: "GET",
        path: "/v1/process/status",
        capability: "processing",
        activation_phase: "P5",
    },
    Endpoint {
        method: "POST",
        path: "/v1/process/cancel",
        capability: "processing",
        activation_phase: "P5",
    },
];
