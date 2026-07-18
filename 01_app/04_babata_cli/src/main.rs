mod app;
mod commands;
mod render;

fn main() {
    if let Err(error) = app::run() {
        if std::env::args().any(|argument| argument == "--json") {
            let application_error = error.downcast_ref::<babata_application::ApplicationError>();
            let code = application_error.map_or("internal", |error| error.code());
            let operation_id = application_error.and_then(|error| error.operation_id());
            println!(
                "{}",
                serde_json::json!({"code":code,"message":error.to_string(),"operation_id":operation_id,"retryable":false,"details":null})
            );
        } else {
            eprintln!("{error}");
        }
        std::process::exit(1);
    }
}
