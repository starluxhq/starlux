pub mod adapters;
pub mod cli;
pub mod providers;
pub mod sink;
pub mod system_prompt;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunRequest {
    pub run_id: String,
    pub conversation_id: String,
    pub provider_id: String,
    pub prompt: String,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    /// `Some(dir)` enables the provider's tools, pinned to that directory.
    /// `None` is chat-only: the CLI cannot touch the filesystem.
    #[serde(default)]
    pub agent_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Context>,
}

/// How much of the model's window this conversation now occupies. Both halves
/// are the provider's own numbers, so unlike a share of the subscription window
/// this is arithmetic rather than a guess: what the CLI says it sent, over what
/// it says the model holds.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Context {
    pub used: u64,
    pub window: u64,
}

/// The provider's view of the user's subscription window — every session they
/// have run, terminal included, not Starlux's share of it. It arrives as a
/// byproduct of a run, so before the first question of a launch the only honest
/// thing to show is the last one seen and how old it is.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateLimit {
    pub provider_id: String,
    /// The provider's own name for the window (`five_hour`, `weekly`, ...),
    /// passed through rather than mapped onto an enum, so a kind we have never
    /// seen still reaches the UI instead of failing the parse.
    pub kind: String,
    pub status: String,
    pub resets_at: Option<i64>,
    pub using_overage: bool,
    /// When Starlux saw this, not when the provider computed it.
    pub observed_at: i64,
}

pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs() as i64)
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum StreamEvent {
    Start {
        run_id: String,
        conversation_id: String,
        provider_id: String,
        prompt: String,
    },
    Chunk {
        run_id: String,
        delta: String,
    },
    Meta {
        run_id: String,
        session_id: Option<String>,
        model: Option<String>,
    },
    End {
        run_id: String,
        text: String,
        session_id: Option<String>,
        usage: Option<Usage>,
    },
    Error {
        run_id: String,
        message: String,
        stderr_tail: String,
    },
    RateLimit {
        run_id: String,
        limit: RateLimit,
    },
}

/// A resolved command line. Kept separate from `tokio::process::Command` so
/// adapters can be unit-tested without spawning anything.
#[derive(Debug, Clone, PartialEq)]
pub struct Invocation {
    pub program: String,
    pub args: Vec<String>,
    pub stdin: Option<String>,
    pub cwd: Option<PathBuf>,
}

#[derive(Debug, Default)]
pub struct ParseState {
    pub session_id: Option<String>,
    pub model: Option<String>,
    pub text: String,
    pub saw_delta: bool,
    pub ended: bool,
}

pub trait CliAdapter: Send + Sync {
    fn invocation(&self, req: &RunRequest) -> Invocation;
    fn parse_line(&self, line: &str, state: &mut ParseState, req: &RunRequest) -> Vec<StreamEvent>;
}
