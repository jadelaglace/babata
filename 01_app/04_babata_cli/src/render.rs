use babata_application::CaptureOutcome;
use babata_infrastructure::{AppConfig, RawStatus};

pub fn render_json(value: &CaptureOutcome) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", serde_json::to_string(value)?);
    Ok(())
}
pub fn render_value<T: serde::Serialize>(
    value: &T,
    json: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if json {
        println!("{}", serde_json::to_string(value)?);
    } else {
        println!("{}", serde_json::to_string_pretty(value)?);
    }
    Ok(())
}
pub fn render_status(config: &AppConfig, status: RawStatus, json: bool) {
    if json {
        println!(
            "{}",
            serde_json::json!({"data_root":config.data_root.0,"reachable":status.reachable,"raw_schema_version":status.schema_version,"pending_journals":status.pending_journals,"orphans":status.orphans})
        );
    } else {
        println!("data root: {}", config.data_root.0.display());
    }
}
