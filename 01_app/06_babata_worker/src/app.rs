#[derive(Debug, Clone)]
pub struct WorkerApp {
    pub enabled: bool,
    pub worker_id: String,
}

pub fn build() -> WorkerApp {
    WorkerApp {
        enabled: false,
        worker_id: "babata-local-worker".to_owned(),
    }
}
