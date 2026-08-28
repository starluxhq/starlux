//! Prints what the model picker would show, and why: which binaries were found
//! on PATH, what each reports about being signed in, and the models it offered.
//!
//! ```sh
//! cargo run --example providers
//! ```
//!
//! The usual reason a provider is missing from the picker is not the picker: it
//! is a stale binary, or a PATH that does not have the CLI on it. This says
//! which, and prints the PATH it searched.
fn main() {
    for provider in starlux_lib::engine::providers::detect() {
        println!(
            "{:<13} {:<12} {:?}",
            provider.id, provider.binary, provider.availability
        );
        for model in &provider.models {
            let efforts = if model.efforts.is_empty() {
                "no thinking levels".to_owned()
            } else {
                model.efforts.join(" ")
            };
            println!("              {:<42} {efforts}", model.id);
        }
        println!();
    }
    println!("PATH={}", std::env::var("PATH").unwrap_or_default());
}
