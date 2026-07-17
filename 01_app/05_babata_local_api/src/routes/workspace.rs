use super::Endpoint;

pub const ENDPOINTS: [Endpoint; 3] = [
    Endpoint {
        method: "POST",
        path: "/v1/workspace/create",
        capability: "workspace.create",
        activation_phase: "P3",
    },
    Endpoint {
        method: "POST",
        path: "/v1/workspace/revise",
        capability: "workspace.revise",
        activation_phase: "P3",
    },
    Endpoint {
        method: "POST",
        path: "/v1/workspace/annotate",
        capability: "workspace.annotate",
        activation_phase: "P3",
    },
];
