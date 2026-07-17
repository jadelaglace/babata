#[derive(Debug, Default, Clone, Copy)]
pub struct WorkerMetrics {
    pub claimed: u64,
    pub completed: u64,
    pub failed: u64,
}
