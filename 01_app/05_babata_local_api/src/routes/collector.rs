use super::Endpoint;

pub const ENDPOINTS: [Endpoint; 5] = [
    Endpoint {
        method: "POST",
        path: "/v1/collector/sessions",
        capability: "collector.start",
        activation_phase: "P4",
    },
    Endpoint {
        method: "GET",
        path: "/v1/collector/candidates",
        capability: "collector.candidates",
        activation_phase: "P4",
    },
    Endpoint {
        method: "POST",
        path: "/v1/collector/select",
        capability: "collector.select",
        activation_phase: "P4",
    },
    Endpoint {
        method: "GET",
        path: "/v1/collector/status",
        capability: "collector.status",
        activation_phase: "P4",
    },
    Endpoint {
        method: "POST",
        path: "/v1/collector/recollect",
        capability: "collector.recollect",
        activation_phase: "P4",
    },
];
