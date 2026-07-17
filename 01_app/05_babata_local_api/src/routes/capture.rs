use super::Endpoint;

pub const ENDPOINTS: [Endpoint; 4] = [
    Endpoint {
        method: "POST",
        path: "/v1/capture/text",
        capability: "capture.text",
        activation_phase: "P3",
    },
    Endpoint {
        method: "POST",
        path: "/v1/capture/file",
        capability: "capture.file",
        activation_phase: "P3",
    },
    Endpoint {
        method: "POST",
        path: "/v1/capture/export",
        capability: "capture.export",
        activation_phase: "P3",
    },
    Endpoint {
        method: "POST",
        path: "/v1/capture/candidate",
        capability: "capture.candidate",
        activation_phase: "P4",
    },
];
