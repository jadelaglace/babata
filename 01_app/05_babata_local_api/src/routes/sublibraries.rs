use super::Endpoint;

pub const ENDPOINTS: [Endpoint; 10] = [
    Endpoint {
        method: "GET",
        path: "/v1/sublibraries",
        capability: "sublibraries.list",
        activation_phase: "P6.3",
    },
    Endpoint {
        method: "POST",
        path: "/v1/sublibraries/create",
        capability: "sublibraries.create",
        activation_phase: "P6.3",
    },
    Endpoint {
        method: "POST",
        path: "/v1/sublibraries/revise",
        capability: "sublibraries.revise",
        activation_phase: "P6.3",
    },
    Endpoint {
        method: "GET",
        path: "/v1/sublibraries/show",
        capability: "sublibraries.show",
        activation_phase: "P6.3",
    },
    Endpoint {
        method: "GET",
        path: "/v1/sublibraries/versions",
        capability: "sublibraries.versions",
        activation_phase: "P6.3",
    },
    Endpoint {
        method: "POST",
        path: "/v1/sublibraries/materialize",
        capability: "sublibraries.materialize",
        activation_phase: "P6.3",
    },
    Endpoint {
        method: "GET",
        path: "/v1/sublibraries/status",
        capability: "sublibraries.status",
        activation_phase: "P6.3",
    },
    Endpoint {
        method: "POST",
        path: "/v1/sublibraries/verify",
        capability: "sublibraries.verify",
        activation_phase: "P6.3",
    },
    Endpoint {
        method: "POST",
        path: "/v1/sublibraries/delete",
        capability: "sublibraries.delete",
        activation_phase: "P6.3",
    },
    Endpoint {
        method: "POST",
        path: "/v1/sublibraries/rebuild",
        capability: "sublibraries.rebuild",
        activation_phase: "P6.3",
    },
];
