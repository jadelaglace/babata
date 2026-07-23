use super::Endpoint;

pub const ENDPOINTS: [Endpoint; 6] = [
    Endpoint {
        method: "GET",
        path: "/v1/outputs",
        capability: "outputs.list",
        activation_phase: "P6.3",
    },
    Endpoint {
        method: "POST",
        path: "/v1/outputs/build",
        capability: "outputs.build",
        activation_phase: "P6.3",
    },
    Endpoint {
        method: "GET",
        path: "/v1/outputs/status",
        capability: "outputs.status",
        activation_phase: "P6.3",
    },
    Endpoint {
        method: "POST",
        path: "/v1/outputs/verify",
        capability: "outputs.verify",
        activation_phase: "P6.3",
    },
    Endpoint {
        method: "POST",
        path: "/v1/outputs/delete",
        capability: "outputs.delete",
        activation_phase: "P6.3",
    },
    Endpoint {
        method: "POST",
        path: "/v1/outputs/rebuild",
        capability: "outputs.rebuild",
        activation_phase: "P6.3",
    },
];
