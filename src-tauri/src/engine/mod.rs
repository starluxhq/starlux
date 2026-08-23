pub mod adapters;
pub mod cli;
pub mod providers;
pub mod sink;
pub mod system_prompt;
pub mod title;

use std::path::{Path, PathBuf};

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
    /// Whether this run may reach the network. The other half of the grant, and
    /// deliberately not a step above `agent_dir`: looking something up should
    /// not cost a folder, and a folder should not quietly buy the network.
    #[serde(default)]
    pub web: bool,
    /// Paths, not contents: this arrives over IPC from a window, and a window
    /// naming a file is not the same as it having handed one over. The core
    /// reads them itself, under its own size cap.
    #[serde(default)]
    pub attachments: Vec<PathBuf>,
}

/// What was attached, as the database and the windows see it: a description,
/// never the contents.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Attachment {
    pub path: String,
    pub name: String,
    pub mime: Option<String>,
    pub bytes: Option<i64>,
}

/// An attachment the core has read. Adapters are handed these rather than
/// paths, which is what keeps `invocation` infallible and unit-testable.
#[derive(Debug, Clone, PartialEq)]
pub struct Loaded {
    pub path: PathBuf,
    pub name: String,
    pub mime: String,
    pub data: Vec<u8>,
}

impl Loaded {
    pub fn described(&self) -> Attachment {
        Attachment {
            path: self.path.to_string_lossy().into_owned(),
            name: self.name.clone(),
            mime: Some(self.mime.clone()),
            bytes: Some(self.data.len() as i64),
        }
    }
}

/// Describes a file without opening it, for the question row that is written
/// before the run starts. A file that has since gone still gets its row: the
/// user attached it, and the run is about to say so itself.
pub fn describe(path: &Path) -> Attachment {
    Attachment {
        path: path.to_string_lossy().into_owned(),
        name: file_name(path),
        mime: Some(mime_of(path).to_owned()),
        bytes: std::fs::metadata(path).ok().map(|meta| meta.len() as i64),
    }
}

pub fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
}

/// Guessed from the extension, because the alternative is sniffing bytes for
/// something only the provider needs to be roughly right about.
pub fn mime_of(path: &Path) -> &'static str {
    let extension = path
        .extension()
        .map(|ext| ext.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    match extension.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "pdf" => "application/pdf",
        "" => "application/octet-stream",
        _ => "text/plain",
    }
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
        attachments: Vec<Attachment>,
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
    /// Added to the child's environment, never replacing it: a CLI that reads
    /// its configuration from one of these still needs the PATH it was found on.
    pub env: Vec<(String, String)>,
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
    fn invocation(&self, req: &RunRequest, files: &[Loaded]) -> Invocation;

    /// A one-shot run that reads the opening question and answers with a name
    /// for the conversation. On the trait rather than inside the Claude adapter
    /// so the next provider gets titles by writing its own argv, not by being
    /// special-cased.
    fn title_invocation(&self, question: &str) -> Invocation;

    fn parse_line(&self, line: &str, state: &mut ParseState, req: &RunRequest) -> Vec<StreamEvent>;
}
