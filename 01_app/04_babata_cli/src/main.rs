mod app;
mod commands;
mod render;

fn main() {
    if let Err(error) = app::run() {
        if std::env::args().any(|argument| argument == "--json") {
            let code = match error.downcast_ref::<babata_application::ApplicationError>() {
                Some(error) => error.code(),
                None => "internal",
            };
            println!(
                "{}",
                serde_json::json!({"code":code,"message":error.to_string(),"operation_id":null,"retryable":false,"details":null})
            );
        } else {
            eprintln!("{error}");
        }
        std::process::exit(1);
    }
}
