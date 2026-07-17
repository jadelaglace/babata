fn main() {
    let descriptor = babata_local_api::build();
    println!(
        "babata local API is disabled ({} declared endpoints)",
        descriptor.endpoints.len()
    );
}
