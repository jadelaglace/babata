use std::net::SocketAddr;

use babata_infrastructure::load_config;
use babata_local_api::{ApiError, ServerConfig};

fn main() {
    if let Err(error) = run() {
        eprintln!("{}: {error}", error.code());
        std::process::exit(1);
    }
}

fn run() -> Result<(), ApiError> {
    let token = std::env::var("BABATA_BROWSER_TOKEN").map_err(|_| {
        ApiError::InvalidRequest("BABATA_BROWSER_TOKEN must be set before starting".to_owned())
    })?;
    let bind_address = std::env::var("BABATA_BROWSER_BIND")
        .unwrap_or_else(|_| "127.0.0.1:43873".to_owned())
        .parse::<SocketAddr>()
        .map_err(|_| ApiError::InvalidRequest("BABATA_BROWSER_BIND is invalid".to_owned()))?;
    let app = load_config().map_err(|error| ApiError::Io(error.to_string()))?;
    babata_local_api::serve(ServerConfig::new(app, bind_address, token)?)
}
