//! The Agent Client Protocol, spoken to opencode over stdio.
//!
//! A second engine beside `cli`, for one reason: `opencode run` does not stream.
//! It prints the whole answer as a single event once the turn is over — in
//! either format, under a pty, and through its own HTTP server — while
//! `opencode acp` sends it in token-sized chunks. Everything that bounds a run
//! is the same agent the CLI path builds; it is chosen over the wire because
//! `opencode acp` takes no `--agent` flag and ignores `default_agent`.

use std::collections::VecDeque;
use std::process::Stdio;
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{ChildStdin, ChildStdout};
use tokio::sync::oneshot;

use super::adapters::opencode;
use super::cli::{self, Runs};
use super::sink::Sink;
use super::{Context, Loaded, RunRequest, StreamEvent, Usage};

const PROTOCOL: u64 = 1;
const STDERR_TAIL_LINES: usize = 20;

pub async fn run(app: tauri::AppHandle, req: RunRequest, sink: Sink) -> Result<(), String> {
    use tauri::Manager;

    let files = cli::load(&req.attachments).await?;
    let invocation = opencode::acp_invocation(&req);

    let mut command = tokio::process::Command::new(&invocation.program);
    command
        .args(&invocation.args)
        .envs(invocation.env.iter().cloned())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    if let Some(cwd) = &invocation.cwd {
        if !cwd.is_dir() {
            return Err(format!(
                "`{}` is no longer a folder. Pick another one for this conversation.",
                cwd.display()
            ));
        }
        command.current_dir(cwd);
    }

    #[cfg(target_os = "windows")]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = command.spawn().map_err(|err| {
        log::warn!(
            "could not start `{} acp`: {err}. PATH={}",
            invocation.program,
            std::env::var("PATH").unwrap_or_default()
        );
        format!(
            "could not start `{}`: {err}. Is it installed and on PATH?",
            invocation.program
        )
    })?;

    let stderr_tail = Arc::new(Mutex::new(VecDeque::<String>::new()));
    if let Some(stderr) = child.stderr.take() {
        let tail = stderr_tail.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let mut tail = tail.lock().unwrap();
                if tail.len() == STDERR_TAIL_LINES {
                    tail.pop_front();
                }
                tail.push_back(line);
            }
        });
    }

    let mut rpc = Rpc {
        stdin: child.stdin.take().expect("stdin piped"),
        lines: BufReader::new(child.stdout.take().expect("stdout piped")).lines(),
        next_id: 0,
    };

    let (cancel_tx, mut cancel_rx) = oneshot::channel();
    app.state::<Runs>().register(req.run_id.clone(), cancel_tx);

    sink.send(StreamEvent::Start {
        run_id: req.run_id.clone(),
        conversation_id: sink.conversation_id(),
        provider_id: req.provider_id.clone(),
        prompt: req.prompt.clone(),
        attachments: files.iter().map(Loaded::described).collect(),
    })?;

    let outcome = converse(&mut rpc, &req, &files, &sink, &mut cancel_rx).await;

    let _ = child.start_kill();
    app.state::<Runs>().finish(&req.run_id);

    match outcome {
        Ok(()) => Ok(()),
        Err(message) => sink.send(StreamEvent::Error {
            run_id: req.run_id.clone(),
            message,
            stderr_tail: tail_text(&stderr_tail),
        }),
    }
}

/// One turn, from handshake to answer. Split out so every early return above it
/// still kills the child and clears the run.
async fn converse(
    rpc: &mut Rpc,
    req: &RunRequest,
    files: &[Loaded],
    sink: &Sink,
    cancel: &mut oneshot::Receiver<()>,
) -> Result<(), String> {
    let mut turn = Turn::new(req.run_id.clone());

    // The filesystem capabilities are declined outright: a client that says it
    // can read files is one the agent may ask to, and chat-only means the
    // question cannot reach them by any route.
    rpc.call(
        "initialize",
        json!({
            "protocolVersion": PROTOCOL,
            "clientCapabilities": { "fs": { "readTextFile": false, "writeTextFile": false } },
        }),
        Listening::Ignored,
        sink,
        &mut turn,
    )
    .await?;

    let cwd = req
        .agent_dir
        .clone()
        .unwrap_or_else(std::env::temp_dir)
        .to_string_lossy()
        .into_owned();

    // Resuming replays the whole conversation back as chunks, which is why
    // nothing is streamed to the window until the prompt has gone out.
    let session = match &req.session_id {
        Some(id) => {
            rpc.call(
                "session/load",
                json!({ "sessionId": id, "cwd": cwd, "mcpServers": [] }),
                Listening::Ignored,
                sink,
                &mut turn,
            )
            .await?;
            id.clone()
        }
        None => {
            let opened = rpc
                .call(
                    "session/new",
                    json!({ "cwd": cwd, "mcpServers": [] }),
                    Listening::Ignored,
                    sink,
                    &mut turn,
                )
                .await?;
            opened
                .get("sessionId")
                .and_then(Value::as_str)
                .ok_or("opencode opened a session without naming it")?
                .to_owned()
        }
    };

    // The agent is chosen here rather than in argv, and it is what carries the
    // permission map and the system prompt. A run that could not select it
    // would be a run bounded by opencode's defaults, so this failing is fatal.
    rpc.set_option(&session, "mode", opencode::CHAT_AGENT, sink, &mut turn)
        .await
        .map_err(|err| format!("could not bound this run to Starlux's agent: {err}"))?;

    if let Some(effort) = &req.effort {
        rpc.set_option(&session, "effort", effort, sink, &mut turn)
            .await?;
    }

    sink.send(StreamEvent::Meta {
        run_id: req.run_id.clone(),
        session_id: Some(session.clone()),
        model: req.model.clone(),
    })?;

    let prompt = json!({ "sessionId": session, "prompt": blocks(req, files) });
    let answered = tokio::select! {
        answered = rpc.call("session/prompt", prompt, Listening::Streamed, sink, &mut turn) => answered?,
        _ = &mut *cancel => {
            // Stopping keeps whatever streamed in rather than discarding it.
            return sink.send(StreamEvent::End {
                run_id: req.run_id.clone(),
                text: turn.text.clone(),
                session_id: Some(session),
                usage: None,
            });
        }
    };

    sink.send(StreamEvent::End {
        run_id: req.run_id.clone(),
        text: turn.text.clone(),
        session_id: Some(session),
        usage: Some(turn.usage(&answered)),
    })
}

/// The prompt as ACP carries it: the question, then whatever was attached.
/// Images go as bytes because that is the block the protocol has for them; a
/// text file goes as text, named, so the model knows which file it is reading.
fn blocks(req: &RunRequest, files: &[Loaded]) -> Vec<Value> {
    use base64::Engine as _;

    let mut blocks = Vec::new();
    if let Some(carried) = super::system_prompt::carried(&req.history) {
        blocks.push(json!({ "type": "text", "text": carried }));
    }
    blocks.push(json!({ "type": "text", "text": req.prompt }));
    for file in files {
        if file.mime.starts_with("image/") {
            blocks.push(json!({
                "type": "image",
                "mimeType": file.mime,
                "data": base64::engine::general_purpose::STANDARD.encode(&file.data),
            }));
        } else {
            blocks.push(json!({
                "type": "text",
                "text": format!("{}:\n{}", file.name, String::from_utf8_lossy(&file.data)),
            }));
        }
    }
    blocks
}

struct Turn {
    run_id: String,
    text: String,
    context: Option<Context>,
    cost_usd: Option<f64>,
}

impl Turn {
    fn new(run_id: String) -> Self {
        Self {
            run_id,
            text: String::new(),
            context: None,
            cost_usd: None,
        }
    }

    /// Reads one update, returning whatever text it added. Everything else it
    /// carries is kept here rather than sent on: a window size is not an
    /// answer, and arrives several times before one.
    fn take(&mut self, update: &Value) -> Option<String> {
        match update.get("sessionUpdate").and_then(Value::as_str) {
            Some("agent_message_chunk") => {
                let delta = update.get("content")?.get("text")?.as_str()?;
                self.text.push_str(delta);
                Some(delta.to_owned())
            }
            Some("usage_update") => {
                if let (Some(used), Some(window)) = (
                    update.get("used").and_then(Value::as_u64),
                    update.get("size").and_then(Value::as_u64),
                ) {
                    self.context = Some(Context { used, window });
                }
                if let Some(cost) = update
                    .get("cost")
                    .and_then(|cost| cost.get("amount"))
                    .and_then(Value::as_f64)
                {
                    self.cost_usd = Some(cost);
                }
                None
            }
            _ => None,
        }
    }

    /// Tokens come from the answer, the window from the last `usage_update`.
    /// opencode reports a window here and nowhere on the CLI path, so a run
    /// through this engine can say how full the context is.
    fn usage(&self, answered: &Value) -> Usage {
        let count = |name: &str| {
            answered
                .get("usage")
                .and_then(|usage| usage.get(name))
                .and_then(Value::as_u64)
                .unwrap_or(0)
        };
        Usage {
            input_tokens: count("inputTokens"),
            output_tokens: count("outputTokens"),
            cost_usd: self.cost_usd,
            context: self.context,
        }
    }
}

/// Whether what arrives while waiting for an answer belongs to this turn.
#[derive(Clone, Copy, PartialEq)]
enum Listening {
    Ignored,
    Streamed,
}

struct Rpc {
    stdin: ChildStdin,
    lines: Lines<BufReader<ChildStdout>>,
    next_id: i64,
}

impl Rpc {
    async fn write(&mut self, message: Value) -> Result<(), String> {
        let line = format!("{message}\n");
        self.stdin
            .write_all(line.as_bytes())
            .await
            .map_err(|err| format!("could not write to opencode: {err}"))?;
        self.stdin
            .flush()
            .await
            .map_err(|err| format!("could not write to opencode: {err}"))
    }

    async fn set_option(
        &mut self,
        session: &str,
        option: &str,
        value: &str,
        sink: &Sink,
        turn: &mut Turn,
    ) -> Result<(), String> {
        self.call(
            "session/set_config_option",
            json!({ "sessionId": session, "configId": option, "value": value }),
            Listening::Ignored,
            sink,
            turn,
        )
        .await
        .map(|_| ())
    }

    /// Sends one request and reads until its answer comes back. Notifications
    /// that arrive meanwhile are this turn's only stream, and a request coming
    /// the other way is answered rather than left pending — an unanswered one
    /// stalls the agent forever.
    async fn call(
        &mut self,
        method: &str,
        params: Value,
        listening: Listening,
        sink: &Sink,
        turn: &mut Turn,
    ) -> Result<Value, String> {
        self.next_id += 1;
        let id = self.next_id;
        self.write(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))
            .await?;

        loop {
            let line = self
                .lines
                .next_line()
                .await
                .map_err(|err| format!("could not read from opencode: {err}"))?
                .ok_or_else(|| format!("opencode stopped before answering `{method}`"))?;

            let Ok(message) = serde_json::from_str::<Value>(&line) else {
                continue;
            };

            if message.get("id").and_then(Value::as_i64) == Some(id) {
                if let Some(error) = message.get("error") {
                    return Err(rpc_error(error));
                }
                return Ok(message.get("result").cloned().unwrap_or(Value::Null));
            }

            match message.get("method").and_then(Value::as_str) {
                Some("session/update") => {
                    if listening == Listening::Streamed {
                        self.update(&message, sink, turn)?;
                    }
                }
                // Anything the agent asks of us is refused: this run declared no
                // tools, so a permission it is asking for is one nobody granted.
                Some(_) => {
                    if let Some(asked) = message.get("id") {
                        let refusal = json!({
                            "jsonrpc": "2.0",
                            "id": asked,
                            "result": { "outcome": { "outcome": "cancelled" } },
                        });
                        self.write(refusal).await?;
                    }
                }
                None => {}
            }
        }
    }

    fn update(&self, message: &Value, sink: &Sink, turn: &mut Turn) -> Result<(), String> {
        let Some(update) = message.get("params").and_then(|p| p.get("update")) else {
            return Ok(());
        };
        let Some(delta) = turn.take(update) else {
            return Ok(());
        };
        sink.send(StreamEvent::Chunk {
            run_id: turn.run_id.clone(),
            delta,
        })
    }
}

fn rpc_error(error: &Value) -> String {
    error
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("opencode reported an error")
        .to_owned()
}

fn tail_text(tail: &Arc<Mutex<VecDeque<String>>>) -> String {
    tail.lock()
        .unwrap()
        .iter()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::Tools;
    use std::path::PathBuf;

    fn request() -> RunRequest {
        RunRequest {
            run_id: "run-1".into(),
            conversation_id: "conv-1".into(),
            provider_id: "opencode-cli".into(),
            prompt: "what colour is this?".into(),
            session_id: None,
            effort: None,
            model: Some("opencode/hy3-free".into()),
            agent_dir: None,
            tools: Tools::default(),
            attachments: Vec::new(),
            history: Vec::new(),
        }
    }

    fn loaded(name: &str, mime: &str, data: &[u8]) -> Loaded {
        Loaded {
            path: PathBuf::from("/tmp").join(name),
            name: name.to_owned(),
            mime: mime.to_owned(),
            data: data.to_vec(),
        }
    }

    fn update(json: Value) -> Value {
        serde_json::json!({ "params": { "update": json } })["params"]["update"].clone()
    }

    #[test]
    fn the_question_leads_and_the_attachments_follow() {
        let req = request();
        let files = [
            loaded("blue.png", "image/png", &[1, 2, 3]),
            loaded("notes.txt", "text/plain", b"remember me"),
        ];
        let blocks = blocks(&req, &files);

        assert_eq!(blocks[0]["type"], "text");
        assert_eq!(blocks[0]["text"], "what colour is this?");
        // Bytes, because that is the block the protocol has for a picture.
        assert_eq!(blocks[1]["type"], "image");
        assert_eq!(blocks[1]["mimeType"], "image/png");
        assert_eq!(blocks[1]["data"], "AQID");
        // Named, so the model knows which file it is being shown.
        assert_eq!(blocks[2]["type"], "text");
        assert_eq!(blocks[2]["text"], "notes.txt:\nremember me");
    }

    #[test]
    fn a_carried_thread_is_its_own_block_ahead_of_the_question() {
        let mut req = request();
        req.history = vec![crate::engine::Past {
            role: "user".into(),
            text: "what is a pulsar?".into(),
        }];
        let blocks = blocks(&req, &[]);
        assert_eq!(blocks.len(), 2);
        assert!(blocks[0]["text"]
            .as_str()
            .unwrap()
            .contains("User: what is a pulsar?"));
        assert_eq!(blocks[1]["text"], req.prompt);
    }

    #[test]
    fn a_question_with_nothing_attached_is_one_block() {
        assert_eq!(blocks(&request(), &[]).len(), 1);
    }

    #[test]
    fn a_chunk_is_the_text_it_carries() {
        let mut turn = Turn::new("run-1".into());
        let first = turn.take(&update(serde_json::json!({
            "sessionUpdate": "agent_message_chunk",
            "content": { "type": "text", "text": "Blue" },
        })));
        assert_eq!(first.as_deref(), Some("Blue"));
        assert_eq!(turn.text, "Blue");

        turn.take(&update(serde_json::json!({
            "sessionUpdate": "agent_message_chunk",
            "content": { "type": "text", "text": ", roughly" },
        })));
        assert_eq!(turn.text, "Blue, roughly");
    }

    /// opencode reports a window here and nowhere on the CLI path, so this is
    /// the only place a run through it can learn how full the context is.
    #[test]
    fn a_usage_update_is_kept_rather_than_streamed() {
        let mut turn = Turn::new("run-1".into());
        let streamed = turn.take(&update(serde_json::json!({
            "sessionUpdate": "usage_update",
            "used": 1921,
            "size": 200000,
            "cost": { "amount": 0.02, "currency": "USD" },
        })));

        assert_eq!(streamed, None);
        assert!(turn.text.is_empty());
        assert_eq!(
            turn.context,
            Some(Context {
                used: 1921,
                window: 200_000
            })
        );
        assert_eq!(turn.cost_usd, Some(0.02));
    }

    #[test]
    fn an_update_it_does_not_know_adds_nothing() {
        let mut turn = Turn::new("run-1".into());
        assert_eq!(
            turn.take(&update(serde_json::json!({ "sessionUpdate": "plan" }))),
            None
        );
        assert_eq!(
            turn.take(&update(serde_json::json!({ "sessionUpdate": "tool_call" }))),
            None
        );
        assert!(turn.text.is_empty());
    }

    /// The tokens are the answer's own; the window came from an update before
    /// it, and both belong to the same turn.
    #[test]
    fn the_ending_totals_the_answer_and_what_was_watched() {
        let mut turn = Turn::new("run-1".into());
        turn.take(&update(serde_json::json!({
            "sessionUpdate": "usage_update", "used": 500, "size": 200000,
        })));
        let usage = turn.usage(&serde_json::json!({
            "stopReason": "end_turn",
            "usage": { "inputTokens": 385, "outputTokens": 2, "totalTokens": 1923 },
        }));

        assert_eq!(usage.input_tokens, 385);
        assert_eq!(usage.output_tokens, 2);
        assert_eq!(
            usage.context,
            Some(Context {
                used: 500,
                window: 200_000
            })
        );
    }

    #[test]
    fn an_answer_that_reports_nothing_still_ends_the_turn() {
        let usage =
            Turn::new("run-1".into()).usage(&serde_json::json!({ "stopReason": "end_turn" }));
        assert_eq!(usage.input_tokens, 0);
        assert_eq!(usage.context, None);
    }

    #[test]
    fn an_error_is_read_by_its_message() {
        assert_eq!(
            rpc_error(&serde_json::json!({ "code": -32602, "message": "Invalid params" })),
            "Invalid params"
        );
        assert_eq!(
            rpc_error(&serde_json::json!({ "code": -32603 })),
            "opencode reported an error"
        );
    }
}
