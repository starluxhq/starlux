pub mod claude;
pub mod opencode;

use super::CliAdapter;

pub fn for_provider(provider_id: &str) -> Option<Box<dyn CliAdapter>> {
    match provider_id {
        "claude-cli" => Some(Box::new(claude::ClaudeAdapter)),
        "opencode-cli" => Some(Box::new(opencode::OpencodeAdapter)),
        _ => None,
    }
}
