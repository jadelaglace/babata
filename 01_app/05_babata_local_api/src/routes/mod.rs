pub mod capture;
pub mod collector;
pub mod explore;
pub mod health;
pub mod outputs;
pub mod process;
pub mod workspace;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Endpoint {
    pub method: &'static str,
    pub path: &'static str,
    pub capability: &'static str,
    pub activation_phase: &'static str,
}

pub fn all() -> Vec<Endpoint> {
    let mut endpoints = Vec::new();
    endpoints.extend(collector::ENDPOINTS);
    endpoints.extend(capture::ENDPOINTS);
    endpoints.extend(workspace::ENDPOINTS);
    endpoints.extend(process::ENDPOINTS);
    endpoints.extend(explore::ENDPOINTS);
    endpoints.extend(outputs::ENDPOINTS);
    endpoints.extend(health::ENDPOINTS);
    endpoints
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    #[test]
    fn endpoint_inventory_has_unique_mappings() {
        let endpoints = super::all();
        let unique = endpoints
            .iter()
            .map(|endpoint| (endpoint.method, endpoint.path))
            .collect::<HashSet<_>>();
        assert_eq!(unique.len(), endpoints.len());
    }
}
