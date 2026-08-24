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
        println!("              models: {:?}\n", provider.models);
    }
    println!("PATH={}", std::env::var("PATH").unwrap_or_default());
}
