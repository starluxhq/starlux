pub mod acp;
pub mod adapters;
pub mod cli;
pub mod providers;
pub mod sink;
pub mod system_prompt;
pub mod title;
pub mod tools;

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

pub use tools::Tools;

/// Which engine drives this run. Two exist because one provider cannot be
/// driven well by reading lines: `opencode run` prints the whole answer at
/// once, and only its ACP mode streams.
pub async fn run(app: tauri::AppHandle, req: RunRequest, sink: sink::Sink) -> Result<(), String> {
    if providers::speaks_acp(&req.provider_id) {
        acp::run(app, req, sink).await
    } else {
        cli::run(app, req, sink).await
    }
}

static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Where the core keeps files it hands to a CLI — a policy a run is bounded by,
/// say. Never the user's own configuration directories, which are theirs.
pub fn set_data_dir(dir: PathBuf) {
    let _ = DATA_DIR.set(dir);
}

pub fn data_dir() -> PathBuf {
    DATA_DIR
        .get()
        .cloned()
        .unwrap_or_else(|| std::env::temp_dir().join("starlux"))
}

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
    /// How hard to think, in the chosen model's own vocabulary. `None` leaves
    /// the provider's default alone, which is the only honest way to say
    /// nothing: the levels differ per model and there is no shared middle.
    #[serde(default)]
    pub effort: Option<String>,
    /// `Some(dir)` enables the provider's tools, pinned to that directory.
    /// `None` is chat-only: the CLI cannot touch the filesystem.
    #[serde(default)]
    pub agent_dir: Option<PathBuf>,
    /// What this run may reach beyond the model itself. Chosen once for the
    /// app rather than per conversation, and deliberately not a step above
    /// `agent_dir`: looking something up should not cost a folder, and a folder
    /// should not quietly buy the network. Overwritten from the database before
    /// the run starts, so a window cannot spend a grant nobody made.
    #[serde(default)]
    pub tools: Tools,
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

/// Where a provider re-sends a whole block each time it grows rather than the
/// piece it added: which block, and where it starts in `text`.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub id: String,
    pub at: usize,
}

#[derive(Debug, Default)]
pub struct ParseState {
    pub session_id: Option<String>,
    pub model: Option<String>,
    pub text: String,
    pub saw_delta: bool,
    pub ended: bool,
    pub block: Option<Block>,
    /// Totalled as the run goes, for providers that report a turn in pieces
    /// rather than once at the end.
    pub usage: Option<Usage>,
}

pub trait CliAdapter: Send + Sync {
    fn invocation(&self, req: &RunRequest, files: &[Loaded]) -> Invocation;

    /// A one-shot run that reads the opening question and answers with a name
    /// for the conversation. On the trait rather than inside the Claude adapter
    /// so the next provider gets titles by writing its own argv, not by being
    /// special-cased.
    ///
    /// `model` is the one the conversation is answering on, offered rather than
    /// imposed: a provider with a small model worth naming picks that instead.
    /// Naming nothing is what an adapter must not do — the CLI then resolves a
    /// model of its own, which is not one the user chose or can be sure of.
    fn title_invocation(&self, question: &str, model: Option<&str>) -> Invocation;

    fn parse_line(&self, line: &str, state: &mut ParseState, req: &RunRequest) -> Vec<StreamEvent>;

    /// Anything the provider needs on disk before it is spawned, written fresh
    /// each time. Gemini's whole grant lives in a file it is pointed at, so a
    /// stale or edited one must never be what a run is bounded by.
    fn prepare(&self, _req: &RunRequest, _files: &[Loaded]) -> Result<(), String> {
        Ok(())
    }

    /// The same, for the cheap run that names a conversation. It goes out
    /// alongside the first question rather than after it, so it cannot wait on
    /// that run having prepared anything.
    fn prepare_title(&self) -> Result<(), String> {
        Ok(())
    }

    /// Called once the CLI's stdout has closed. Providers whose stream carries a
    /// final event have nothing to do here; those that simply stop talking end
    /// the turn from this. Returning nothing leaves `cli` to report the exit,
    /// which is what a run that produced no answer should say.
    fn finish(&self, _state: &mut ParseState, _req: &RunRequest) -> Vec<StreamEvent> {
        Vec::new()
    }

    /// What the naming run actually said, pulled out of whatever it printed.
    /// Plain-text output needs no unwrapping; a CLI that only speaks JSON does.
    fn title_text(&self, stdout: &str) -> String {
        stdout.to_owned()
    }
}
