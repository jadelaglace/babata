mod app;
mod lease;
mod metrics;
mod runner;
mod shutdown;

fn main() {
    let worker = app::build();
    let metrics = metrics::WorkerMetrics::default();
    let mut shutdown = shutdown::ShutdownSignal::default();
    shutdown.shutdown();
    let lifecycle_disabled = runner::run(&worker).is_err()
        && runner::claim_once(&worker).is_err()
        && lease::heartbeat(&worker.worker_id).is_err();
    println!(
        "babata worker is disabled: {}; id={}; lifecycle_disabled={}; shutdown={}; metrics={}/{}/{}",
        !worker.enabled,
        worker.worker_id,
        lifecycle_disabled,
        shutdown.is_requested(),
        metrics.claimed,
        metrics.completed,
        metrics.failed
    );
}
