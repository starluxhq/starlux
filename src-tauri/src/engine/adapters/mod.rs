pub mod claude;

use super::CliAdapter;

pub fn for_provider(provider_id: &str) -> Option<Box<dyn CliAdapter>> {
    match provider_id {
        "claude-cli" => Some(Box::new(claude::ClaudeAdapter)),
        _ => None,
    }
}
