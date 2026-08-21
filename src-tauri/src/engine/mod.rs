pub mod adapters;
pub mod cli;
pub mod providers;

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunRequest {
    pub run_id: String,
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
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum StreamEvent {
    Start {
        run_id: String,
        provider_id: String,
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
