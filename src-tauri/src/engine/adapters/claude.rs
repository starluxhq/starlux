use base64::Engine as _;
use serde_json::Value;

use crate::engine::tools::{Tools, WEB_FETCH, WEB_SEARCH};
use crate::engine::{
    now, system_prompt, CliAdapter, Context, Invocation, Loaded, ParseState, RateLimit, RunRequest,
    StreamEvent, Usage,
};

/// Claude's names for the tools Starlux grants. Named exactly, not by category:
/// an allowlist that grew with the provider would hand over whatever it added
/// next.
const NAMES: [(&str, &str); 2] = [(WEB_SEARCH, "WebSearch"), (WEB_FETCH, "WebFetch")];

/// Namespaced so a user's own agent of the same name is never the one that runs.
const CHAT_AGENT: &str = "starlux-chat";
const TITLE_AGENT: &str = "starlux-title";

/// Naming a conversation is not the work the user is paying attention to, so
/// it is asked of the cheapest model rather than the one they picked.
const TITLE_MODEL: &str = "haiku";

pub struct ClaudeAdapter;

impl CliAdapter for ClaudeAdapter {
    fn invocation(&self, req: &RunRequest, files: &[Loaded]) -> Invocation {
        let mut args = vec![
            "-p".into(),
            "--output-format".into(),
            "stream-json".into(),
            "--include-partial-messages".into(),
            "--verbose".into(),
        ];

        if let Some(model) = &req.model {
            args.push("--model".into());
            args.push(model.clone());
        }

        if let Some(session) = &req.session_id {
            args.push("--resume".into());
            args.push(session.clone());
        }
        // The CLI validates this itself and warns rather than fails on a name
        // it does not know, so a level from a stale picker costs the default
        // effort and not the run.
        if let Some(effort) = &req.effort {
            args.push("--effort".into());
            args.push(effort.clone());
        }

        // Chat-only unless the conversation opted into agent mode. `--bare` is
        // deliberately not used: it forces ANTHROPIC_API_KEY auth, which would
        // bypass the subscription this bridge exists to use.
        //
        // A session-scoped agent is what takes the tools away. `--allowed-tools ""`
        // reads as "nothing further is pre-approved" rather than "no tools", and a
        // denylist only covers the tools that existed when it was written — asked to
        // read a file with Read, Bash and Glob denied, the CLI reached it through
        // another tool. An agent declaring no tools has none to reach for, and MCP
        // servers are excluded because they are tools the user configured elsewhere.
        //
        // Its prompt also replaces the provider's, dropping a coding-agent preamble
        // that costs around 25k input tokens a turn. Agent mode still needs that
        // preamble, so it appends instead.
        //
        // Two things decide what a run can do and both are needed: the agent's
        // `tools` array is what makes a tool exist, and `--allowedTools` is what
        // pre-approves it. Declared but unapproved, the call comes back denied —
        // and a launcher has nowhere to show the prompt that would approve it.
        if req.agent_dir.is_none() {
            args.push("--strict-mcp-config".into());
            args.push("--agents".into());
            args.push(chat_agent(&req.tools));
            args.push("--agent".into());
            args.push(CHAT_AGENT.into());
        } else {
            args.push("--append-system-prompt".into());
            args.push(system_prompt::agent());
            // A launcher has nowhere to put an approval prompt, and a run with
            // no way to answer one has every edit denied. Choosing the folder is
            // the grant; whatever the user's own CLI settings refuse still is.
            args.push("--permission-mode".into());
            args.push("acceptEdits".into());
        }

        let granted = granted(&req.tools);
        if !granted.is_empty() {
            args.push("--allowedTools".into());
            args.extend(granted.iter().map(|tool| (*tool).to_owned()));
        }

        // Plain text on stdin is the whole prompt when nothing is attached.
        // Files need the richer input, which wraps every question in a JSON
        // envelope, so it is switched on only where it buys something.
        let stdin = if files.is_empty() {
            req.prompt.clone()
        } else {
            args.push("--input-format".into());
            args.push("stream-json".into());
            user_message(&req.prompt, files)
        };

        Invocation {
            program: "claude".into(),
            args,
            stdin: Some(stdin),
            cwd: req.agent_dir.clone(),
            env: Vec::new(),
        }
    }

    /// Plain text out, no session in: resuming would carry the whole thread and
    /// cost exactly the tokens this run exists to avoid. No `cwd` either — it
    /// reads what it is given on stdin and has no reason to see a disk.
    fn title_invocation(&self, question: &str) -> Invocation {
        Invocation {
            program: "claude".into(),
            args: vec![
                "-p".into(),
                "--output-format".into(),
                "text".into(),
                "--model".into(),
                TITLE_MODEL.into(),
                "--strict-mcp-config".into(),
                "--agents".into(),
                title_agent(),
                "--agent".into(),
                TITLE_AGENT.into(),
            ],
            stdin: Some(question.to_owned()),
            cwd: None,
            env: Vec::new(),
        }
    }

    fn parse_line(&self, line: &str, state: &mut ParseState, req: &RunRequest) -> Vec<StreamEvent> {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            return Vec::new();
        };

        let mut events = Vec::new();
        let run_id = req.run_id.clone();

        let mut meta_changed = false;
        if state.session_id.is_none() {
            if let Some(session) = value.get("session_id").and_then(Value::as_str) {
                state.session_id = Some(session.to_owned());
                meta_changed = true;
            }
        }
        if state.model.is_none() {
            if let Some(model) = value.get("model").and_then(Value::as_str) {
                state.model = Some(model.to_owned());
                meta_changed = true;
            }
        }
        if meta_changed {
            events.push(StreamEvent::Meta {
                run_id: run_id.clone(),
                session_id: state.session_id.clone(),
                model: state.model.clone(),
            });
        }

        match value.get("type").and_then(Value::as_str) {
            Some("stream_event") => {
                if let Some(text) = text_delta(&value) {
                    state.saw_delta = true;
                    state.text.push_str(text);
                    events.push(StreamEvent::Chunk {
                        run_id,
                        delta: text.to_owned(),
                    });
                }
            }
            // Only a fallback: with --include-partial-messages the deltas above
            // already carry this text, and emitting both would duplicate it.
            Some("assistant") if !state.saw_delta => {
                if let Some(text) = assistant_text(&value) {
                    state.text.push_str(&text);
                    events.push(StreamEvent::Chunk {
                        run_id,
                        delta: text,
                    });
                }
            }
            // Not ours and not asked for: the CLI reports the subscription
            // window on its own, and we were dropping it on the floor.
            Some("rate_limit_event") => {
                if let Some(limit) = rate_limit(&value, &req.provider_id) {
                    events.push(StreamEvent::RateLimit { run_id, limit });
                }
            }
            Some("result") => {
                state.ended = true;
                let is_error = value
                    .get("is_error")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);

                let result_text = value.get("result").and_then(Value::as_str);

                if is_error {
                    events.push(StreamEvent::Error {
                        run_id,
                        message: result_text
                            .unwrap_or("the provider reported an error")
                            .into(),
                        stderr_tail: String::new(),
                    });
                } else {
                    let text = if state.text.is_empty() {
                        result_text.unwrap_or_default().to_owned()
                    } else {
                        state.text.clone()
                    };
                    events.push(StreamEvent::End {
                        run_id,
                        text,
                        session_id: state.session_id.clone(),
                        usage: usage(&value, state.model.as_deref()),
                    });
                }
            }
            _ => {}
        }

        events
    }
}

/// One JSONL line carrying the question and everything attached to it. Images
/// go as base64 blocks; anything else goes as text under its own name, because
/// a chat-only run has no tool with which to open a path.
fn user_message(prompt: &str, files: &[Loaded]) -> String {
    let mut content = vec![serde_json::json!({ "type": "text", "text": prompt })];

    for file in files {
        content.push(match image_type(&file.mime) {
            Some(media_type) => serde_json::json!({
                "type": "image",
                "source": {
                    "type": "base64",
                    "media_type": media_type,
                    "data": base64::engine::general_purpose::STANDARD.encode(&file.data),
                },
            }),
            None => serde_json::json!({
                "type": "text",
                "text": format!(
                    "Attached file `{}`:\n\n{}",
                    file.name,
                    String::from_utf8_lossy(&file.data)
                ),
            }),
        });
    }

    format!(
        "{}\n",
        serde_json::json!({
            "type": "user",
            "message": { "role": "user", "content": content },
        })
    )
}

/// The four the API takes. A `.bmp` is an image to the user and a text block to
/// the model, which is the honest failure: it arrives named, and unreadable.
fn image_type(mime: &str) -> Option<&'static str> {
    match mime {
        "image/png" => Some("image/png"),
        "image/jpeg" => Some("image/jpeg"),
        "image/gif" => Some("image/gif"),
        "image/webp" => Some("image/webp"),
        _ => None,
    }
}

/// Only the tools actually granted, under the names this CLI knows them by.
fn granted(tools: &Tools) -> Vec<&'static str> {
    NAMES
        .iter()
        .filter(|(id, _)| tools.get(id))
        .map(|(_, name)| *name)
        .collect()
}

fn chat_agent(tools: &Tools) -> String {
    serde_json::json!({
        CHAT_AGENT: {
            "description": "Answers questions from the Starlux bar",
            "prompt": system_prompt::chat(),
            "tools": granted(tools),
        }
    })
    .to_string()
}

fn title_agent() -> String {
    serde_json::json!({
        TITLE_AGENT: {
            "description": "Names a Starlux conversation",
            "prompt": system_prompt::title(),
            "tools": [],
        }
    })
    .to_string()
}

fn text_delta(value: &Value) -> Option<&str> {
    let event = value.get("event")?;
    if event.get("type")?.as_str()? != "content_block_delta" {
        return None;
    }
    let delta = event.get("delta")?;
    // Thinking and tool-input deltas share this envelope; only text is answer text.
    if delta.get("type")?.as_str()? != "text_delta" {
        return None;
    }
    delta.get("text")?.as_str()
}

fn assistant_text(value: &Value) -> Option<String> {
    let blocks = value.get("message")?.get("content")?.as_array()?;
    let text: String = blocks
        .iter()
        .filter(|block| block.get("type").and_then(Value::as_str) == Some("text"))
        .filter_map(|block| block.get("text").and_then(Value::as_str))
        .collect();
    (!text.is_empty()).then_some(text)
}

fn rate_limit(value: &Value, provider_id: &str) -> Option<RateLimit> {
    let info = value.get("rate_limit_info")?;
    Some(RateLimit {
        provider_id: provider_id.to_owned(),
        kind: info
            .get("rateLimitType")
            .and_then(Value::as_str)?
            .to_owned(),
        status: info
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_owned(),
        resets_at: info.get("resetsAt").and_then(Value::as_i64),
        using_overage: info
            .get("isUsingOverage")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        observed_at: now(),
    })
}

/// `modelUsage` is keyed by model, and in agent mode a subagent's model can
/// appear beside the one that answered. The conversation's context is the
/// answering model's, so a report we cannot attribute is left unread.
fn context(value: &Value, model: Option<&str>) -> Option<Context> {
    let per_model = value.get("modelUsage")?.as_object()?;
    let entry = match model.and_then(|model| per_model.get(model)) {
        Some(entry) => entry,
        None if per_model.len() == 1 => per_model.values().next()?,
        None => return None,
    };

    let window = entry.get("contextWindow").and_then(Value::as_u64)?;
    let count = |name| entry.get(name).and_then(Value::as_u64).unwrap_or(0);
    // What the next turn will carry: this turn's new input, everything replayed
    // from cache, and the answer just written.
    let used = count("inputTokens")
        + count("cacheReadInputTokens")
        + count("cacheCreationInputTokens")
        + count("outputTokens");

    (window > 0).then_some(Context { used, window })
}

fn usage(value: &Value, model: Option<&str>) -> Option<Usage> {
    let usage = value.get("usage")?;
    Some(Usage {
        input_tokens: usage
            .get("input_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        output_tokens: usage
            .get("output_tokens")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        cost_usd: value.get("total_cost_usd").and_then(Value::as_f64),
        context: context(value, model),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const FIXTURE: &str = include_str!("../../../tests/fixtures/claude-stream.ndjson");
    const SESSION: &str = "00000000-1111-2222-3333-444444444444";

    fn request() -> RunRequest {
        RunRequest {
            run_id: "run-1".into(),
            conversation_id: "conv-1".into(),
            provider_id: "claude-cli".into(),
            prompt: "count to 3".into(),
            session_id: None,
            effort: None,
            model: None,
            agent_dir: None,
            tools: Tools::default(),
            attachments: Vec::new(),
        }
    }

    fn loaded(name: &str, data: &[u8]) -> Loaded {
        let path = PathBuf::from("/tmp").join(name);
        Loaded {
            mime: crate::engine::mime_of(&path).to_owned(),
            name: name.to_owned(),
            path,
            data: data.to_vec(),
        }
    }

    fn blocks(invocation: &Invocation) -> Vec<Value> {
        let line: Value = serde_json::from_str(invocation.stdin.as_deref().unwrap()).unwrap();
        line["message"]["content"].as_array().unwrap().clone()
    }

    fn drain(lines: &[&str]) -> (Vec<StreamEvent>, ParseState) {
        let adapter = ClaudeAdapter;
        let req = request();
        let mut state = ParseState::default();
        let events = lines
            .iter()
            .flat_map(|line| adapter.parse_line(line, &mut state, &req))
            .collect();
        (events, state)
    }

    fn chunks(events: &[StreamEvent]) -> String {
        events
            .iter()
            .filter_map(|event| match event {
                StreamEvent::Chunk { delta, .. } => Some(delta.as_str()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn streams_answer_text_from_deltas() {
        let lines: Vec<&str> = FIXTURE.lines().collect();
        let (events, _) = drain(&lines);

        assert_eq!(chunks(&events), "1, 2, 3");

        let end = events
            .iter()
            .find_map(|event| match event {
                StreamEvent::End {
                    text,
                    session_id,
                    usage,
                    ..
                } => Some((text, session_id, usage)),
                _ => None,
            })
            .expect("result line should produce an End");

        assert_eq!(end.0, "1, 2, 3");
        assert_eq!(end.1.as_deref(), Some(SESSION));
        let usage = end.2.as_ref().expect("result carries usage");
        assert_eq!(usage.output_tokens, 9);
        assert_eq!(usage.cost_usd, Some(0.0858444));
    }

    #[test]
    fn reports_session_and_model_once() {
        let lines: Vec<&str> = FIXTURE.lines().collect();
        let (events, _) = drain(&lines);

        let metas: Vec<_> = events
            .iter()
            .filter(|event| matches!(event, StreamEvent::Meta { .. }))
            .collect();

        // session_id arrives on the first line, model only on system/init.
        assert_eq!(metas.len(), 2);
        assert!(matches!(
            metas[1],
            StreamEvent::Meta {
                model: Some(model),
                ..
            } if model == "claude-sonnet-5"
        ));
    }

    #[test]
    fn assistant_message_does_not_duplicate_streamed_text() {
        let lines: Vec<&str> = FIXTURE.lines().collect();
        let (events, _) = drain(&lines);
        assert_eq!(chunks(&events).matches("1, 2, 3").count(), 1);
    }

    #[test]
    fn falls_back_to_assistant_message_without_deltas() {
        let (events, _) = drain(&[
            r#"{"type":"assistant","message":{"content":[{"type":"text","text":"hello"}]}}"#,
            r#"{"type":"result","subtype":"success","is_error":false,"result":"hello"}"#,
        ]);
        assert_eq!(chunks(&events), "hello");
    }

    #[test]
    fn ignores_thinking_and_tool_input_deltas() {
        let (events, _) = drain(&[
            r#"{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"thinking_delta","thinking":"hmm"}}}"#,
            r#"{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"input_json_delta","partial_json":"{\"a\":"}}}"#,
            r#"{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"text_delta","text":"visible"}}}"#,
        ]);
        assert_eq!(chunks(&events), "visible");
    }

    #[test]
    fn survives_truncated_and_unparseable_lines() {
        let (events, state) = drain(&[
            "",
            "not json at all",
            r#"{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"text_"#,
            r#"{"type":"stream_event","event":{"type":"content_block_delta","delta":{"type":"text_delta","text":"ok"}}}"#,
        ]);
        assert_eq!(chunks(&events), "ok");
        assert!(!state.ended);
    }

    fn limits(events: &[StreamEvent]) -> Vec<&RateLimit> {
        events
            .iter()
            .filter_map(|event| match event {
                StreamEvent::RateLimit { limit, .. } => Some(limit),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn reports_the_subscription_window_the_cli_volunteers() {
        let (events, _) = drain(&[
            r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed","resetsAt":1787421000,"rateLimitType":"five_hour","overageStatus":"rejected","overageDisabledReason":"org_level_disabled","isUsingOverage":false}}"#,
        ]);

        let limit = limits(&events)[0];
        assert_eq!(limit.provider_id, "claude-cli");
        assert_eq!(limit.kind, "five_hour");
        assert_eq!(limit.status, "allowed");
        assert_eq!(limit.resets_at, Some(1787421000));
        assert!(!limit.using_overage);
        assert!(limit.observed_at > 0);
    }

    /// A window kind Anthropic adds later must reach the UI, which shows what
    /// it does not recognise verbatim rather than hiding it.
    #[test]
    fn passes_through_a_window_kind_it_has_never_seen() {
        let (events, _) = drain(&[
            r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed_warning","rateLimitType":"lunar_cycle","isUsingOverage":true}}"#,
        ]);

        let limit = limits(&events)[0];
        assert_eq!(limit.kind, "lunar_cycle");
        assert_eq!(limit.status, "allowed_warning");
        assert_eq!(limit.resets_at, None);
        assert!(limit.using_overage);
    }

    #[test]
    fn a_rate_limit_event_without_a_window_kind_is_ignored() {
        let (events, state) = drain(&[
            r#"{"type":"rate_limit_event"}"#,
            r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed"}}"#,
        ]);
        assert!(limits(&events).is_empty());
        assert!(!state.ended);
    }

    fn ending(events: &[StreamEvent]) -> Option<&Usage> {
        events.iter().find_map(|event| match event {
            StreamEvent::End { usage, .. } => usage.as_ref(),
            _ => None,
        })
    }

    fn result_with(model_usage: &str) -> String {
        format!(
            r#"{{"type":"result","is_error":false,"result":"hi",
                 "usage":{{"input_tokens":9,"output_tokens":69}},
                 "modelUsage":{model_usage}}}"#
        )
    }

    const HAIKU: &str = "claude-haiku-4-5-20251001";

    #[test]
    fn reports_how_full_the_context_is() {
        let entry = format!(
            r#"{{"{HAIKU}":{{"inputTokens":9,"cacheReadInputTokens":18066,
                 "cacheCreationInputTokens":8685,"outputTokens":69,"contextWindow":200000}}}}"#
        );
        let (events, _) = drain(&[
            &format!(r#"{{"type":"system","model":"{HAIKU}"}}"#),
            &result_with(&entry),
        ]);

        // Everything the next turn carries, not just the tokens this one added.
        assert_eq!(
            ending(&events).unwrap().context,
            Some(Context {
                used: 26_829,
                window: 200_000
            })
        );
    }

    #[test]
    fn attributes_the_context_to_the_model_that_answered() {
        let entry = format!(
            r#"{{"{HAIKU}":{{"inputTokens":10,"contextWindow":200000}},
                 "claude-opus-5":{{"inputTokens":999,"contextWindow":500000}}}}"#
        );
        let (events, _) = drain(&[
            &format!(r#"{{"type":"system","model":"{HAIKU}"}}"#),
            &result_with(&entry),
        ]);

        let context = ending(&events).unwrap().context.unwrap();
        assert_eq!(context.window, 200_000);
    }

    #[test]
    fn a_lone_report_is_read_even_when_the_model_went_unnamed() {
        let (events, _) = drain(&[&result_with(
            r#"{"some-model":{"inputTokens":7,"contextWindow":1000}}"#,
        )]);
        assert_eq!(
            ending(&events).unwrap().context,
            Some(Context {
                used: 7,
                window: 1000
            })
        );
    }

    #[test]
    fn a_report_it_cannot_attribute_is_left_unread() {
        let (events, _) = drain(&[&result_with(
            r#"{"one":{"inputTokens":1,"contextWindow":1000},
                "other":{"inputTokens":2,"contextWindow":2000}}"#,
        )]);
        assert_eq!(ending(&events).unwrap().context, None);
    }

    #[test]
    fn a_window_of_zero_is_not_a_fullness_of_infinity() {
        let (events, _) = drain(&[&result_with(
            r#"{"one":{"inputTokens":5,"contextWindow":0}}"#,
        )]);
        assert_eq!(ending(&events).unwrap().context, None);
    }

    #[test]
    fn a_run_that_reports_no_model_usage_reports_no_context() {
        let lines: Vec<&str> = FIXTURE.lines().collect();
        let (events, _) = drain(&lines);
        let usage = ending(&events).unwrap();
        assert_eq!(usage.context, None);
        assert_eq!(usage.output_tokens, 9);
    }

    #[test]
    fn surfaces_error_results() {
        let (events, state) = drain(&[
            r#"{"type":"result","subtype":"error_during_execution","is_error":true,"result":"Credit balance is too low"}"#,
        ]);
        assert!(state.ended);
        assert!(matches!(
            events.last(),
            Some(StreamEvent::Error { message, .. }) if message == "Credit balance is too low"
        ));
    }

    #[test]
    fn chat_only_hands_the_run_no_tools_at_all() {
        let invocation = ClaudeAdapter.invocation(&request(), &[]);
        assert_eq!(invocation.program, "claude");
        assert_eq!(invocation.stdin.as_deref(), Some("count to 3"));
        assert_eq!(invocation.cwd, None);
        assert!(!invocation.args.iter().any(|arg| arg == "--bare"));

        let definition = invocation
            .args
            .iter()
            .position(|arg| arg == "--agents")
            .expect("chat-only runs must define the agent they run as");
        let agents: Value = serde_json::from_str(&invocation.args[definition + 1]).unwrap();
        let agent = &agents[CHAT_AGENT];
        assert_eq!(agent["tools"], serde_json::json!([]));
        assert!(agent["prompt"]
            .as_str()
            .unwrap()
            .starts_with("You are Starlux"));

        let selected = invocation
            .args
            .iter()
            .position(|arg| arg == "--agent")
            .expect("defining the agent does not select it");
        assert_eq!(invocation.args[selected + 1], CHAT_AGENT);

        // Servers configured elsewhere would arrive as tools the agent never declared.
        assert!(invocation
            .args
            .iter()
            .any(|arg| arg == "--strict-mcp-config"));
        // Appending would keep the preamble the agent's own prompt exists to drop.
        assert!(!invocation
            .args
            .iter()
            .any(|arg| arg == "--append-system-prompt"));
    }

    #[test]
    fn naming_a_conversation_is_a_cheap_toolless_one_shot() {
        let invocation = ClaudeAdapter.title_invocation("what is a spectral class?");
        assert_eq!(invocation.program, "claude");
        assert_eq!(invocation.cwd, None);
        assert_eq!(
            invocation.stdin.as_deref(),
            Some("what is a spectral class?")
        );
        assert!(!invocation.args.iter().any(|arg| arg == "--bare"));

        let model = invocation
            .args
            .iter()
            .position(|arg| arg == "--model")
            .expect("naming must pin its own model, not inherit the user's");
        assert_eq!(invocation.args[model + 1], TITLE_MODEL);

        // Resuming would carry the whole thread and cost what this run avoids.
        assert!(!invocation.args.iter().any(|arg| arg == "--resume"));

        let definition = invocation
            .args
            .iter()
            .position(|arg| arg == "--agents")
            .expect("naming must define the agent it runs as");
        let agents: Value = serde_json::from_str(&invocation.args[definition + 1]).unwrap();
        assert_eq!(agents[TITLE_AGENT]["tools"], serde_json::json!([]));
        assert!(invocation
            .args
            .iter()
            .any(|arg| arg == "--strict-mcp-config"));
    }

    #[test]
    fn agent_mode_appends_to_the_provider_prompt() {
        let mut req = request();
        req.agent_dir = Some(PathBuf::from("/tmp/project"));
        let invocation = ClaudeAdapter.invocation(&req, &[]);

        let prompt = invocation
            .args
            .iter()
            .position(|arg| arg == "--append-system-prompt")
            .expect("agent mode must keep the provider's tool instructions");
        assert!(invocation.args[prompt + 1].contains("starlux-widget"));
        assert!(!invocation.args.iter().any(|arg| arg == "--system-prompt"));
    }

    #[test]
    fn agent_mode_enables_tools_pinned_to_a_directory() {
        let mut req = request();
        req.agent_dir = Some(PathBuf::from("/tmp/project"));
        let invocation = ClaudeAdapter.invocation(&req, &[]);

        assert_eq!(invocation.cwd, Some(PathBuf::from("/tmp/project")));

        let mode = invocation
            .args
            .iter()
            .position(|arg| arg == "--permission-mode")
            .expect("agent mode must accept edits it cannot ask about");
        assert_eq!(invocation.args[mode + 1], "acceptEdits");
    }

    fn tools_of(invocation: &Invocation) -> Value {
        let at = invocation
            .args
            .iter()
            .position(|arg| arg == "--agents")
            .expect("chat-only runs define the agent they run as");
        let agents: Value = serde_json::from_str(&invocation.args[at + 1]).unwrap();
        agents[CHAT_AGENT]["tools"].clone()
    }

    fn allowlist(invocation: &Invocation) -> Option<&[String]> {
        let at = invocation
            .args
            .iter()
            .position(|arg| arg == "--allowedTools" || arg == "--allowed-tools")?;
        Some(&invocation.args[at + 1..])
    }

    /// The tools array is what makes them exist; the flag is what pre-approves
    /// them. Without the second the calls come back denied, and a launcher has
    /// nowhere to show the prompt that would answer that.
    #[test]
    fn a_granted_tool_is_both_declared_and_approved() {
        let mut req = request();
        req.tools = Tools {
            web_search: true,
            web_fetch: true,
        };
        let invocation = ClaudeAdapter.invocation(&req, &[]);

        assert_eq!(
            tools_of(&invocation),
            serde_json::json!(["WebSearch", "WebFetch"])
        );
        assert_eq!(
            allowlist(&invocation),
            Some(&["WebSearch".to_owned(), "WebFetch".to_owned()][..])
        );
        // The network is not a filesystem: the run still has neither cwd nor
        // a tool that could reach one.
        assert_eq!(invocation.cwd, None);
    }

    /// The toggles are separate because the tools are, and granting one must
    /// not carry the other in with it.
    #[test]
    fn each_tool_is_granted_on_its_own() {
        let mut req = request();
        req.tools.web_fetch = true;
        let invocation = ClaudeAdapter.invocation(&req, &[]);

        assert_eq!(tools_of(&invocation), serde_json::json!(["WebFetch"]));
        assert_eq!(allowlist(&invocation), Some(&["WebFetch".to_owned()][..]));
    }

    #[test]
    fn without_a_grant_the_run_has_no_tools_and_approves_nothing() {
        let invocation = ClaudeAdapter.invocation(&request(), &[]);
        assert_eq!(tools_of(&invocation), serde_json::json!([]));
        assert_eq!(allowlist(&invocation), None);
    }

    /// Two grants, not a ladder: opting into a folder must not silently add
    /// the network, and adding the network must not need one.
    #[test]
    fn agent_mode_reaches_the_web_only_when_that_was_granted_too() {
        let mut req = request();
        req.agent_dir = Some(PathBuf::from("/tmp/project"));
        assert_eq!(allowlist(&ClaudeAdapter.invocation(&req, &[])), None);

        req.tools = Tools {
            web_search: true,
            web_fetch: true,
        };
        let invocation = ClaudeAdapter.invocation(&req, &[]);
        assert_eq!(
            allowlist(&invocation),
            Some(&["WebSearch".to_owned(), "WebFetch".to_owned()][..])
        );
        assert_eq!(invocation.cwd, Some(PathBuf::from("/tmp/project")));
    }

    /// Naming a conversation is a one-shot on the question text; nothing about
    /// it should ever reach the network.
    #[test]
    fn naming_a_conversation_never_gains_a_tool() {
        let invocation = ClaudeAdapter.title_invocation("what is a spectral class?");
        assert_eq!(allowlist(&invocation), None);
    }

    #[test]
    fn chat_only_never_relaxes_permissions() {
        let invocation = ClaudeAdapter.invocation(&request(), &[]);
        assert!(invocation.cwd.is_none());
        assert!(!invocation.args.iter().any(|arg| arg == "--permission-mode"));
    }

    #[test]
    fn an_image_is_sent_as_a_block_the_model_can_see() {
        let invocation =
            ClaudeAdapter.invocation(&request(), &[loaded("blue.png", &[0x89, b'P', b'N'])]);

        // The richer input format costs an envelope, so it is only asked for
        // when there is something in it that plain text cannot carry.
        let format = invocation
            .args
            .iter()
            .position(|arg| arg == "--input-format")
            .expect("attachments need stream-json in as well as out");
        assert_eq!(invocation.args[format + 1], "stream-json");

        let blocks = blocks(&invocation);
        assert_eq!(
            blocks[0],
            serde_json::json!({"type":"text","text":"count to 3"})
        );
        assert_eq!(
            blocks[1],
            serde_json::json!({
                "type": "image",
                "source": {"type": "base64", "media_type": "image/png", "data": "iVBO"},
            })
        );
    }

    /// A chat-only run has no tool with which to open a path, so a text file
    /// arrives as its contents or not at all.
    #[test]
    fn a_text_file_arrives_as_text_under_its_own_name() {
        let invocation = ClaudeAdapter.invocation(&request(), &[loaded("notes.md", b"# Orbits")]);
        let blocks = blocks(&invocation);
        assert_eq!(blocks[1]["type"], "text");
        let text = blocks[1]["text"].as_str().unwrap();
        assert!(text.contains("notes.md"));
        assert!(text.ends_with("# Orbits"));
    }

    /// The user sees an image; the model sees a text block it cannot read. The
    /// alternative is a media type the API rejects, taking the question with it.
    #[test]
    fn an_image_type_the_api_does_not_take_is_not_claimed_to_be_one() {
        let invocation = ClaudeAdapter.invocation(&request(), &[loaded("scan.bmp", b"BM")]);
        assert_eq!(blocks(&invocation)[1]["type"], "text");
    }

    #[test]
    fn a_question_with_nothing_attached_is_still_plain_text_on_stdin() {
        let invocation = ClaudeAdapter.invocation(&request(), &[]);
        assert_eq!(invocation.stdin.as_deref(), Some("count to 3"));
        assert!(!invocation.args.iter().any(|arg| arg == "--input-format"));
    }

    #[test]
    fn attachments_survive_alongside_agent_mode_and_a_resumed_session() {
        let mut req = request();
        req.agent_dir = Some(PathBuf::from("/tmp/project"));
        req.session_id = Some(SESSION.into());
        let invocation = ClaudeAdapter.invocation(&req, &[loaded("a.png", b"x")]);

        assert_eq!(blocks(&invocation).len(), 2);
        assert!(invocation.args.iter().any(|arg| arg == "--resume"));
        assert_eq!(invocation.cwd, Some(PathBuf::from("/tmp/project")));
    }

    #[test]
    fn resumes_the_provider_session_for_multi_turn() {
        let mut req = request();
        req.session_id = Some(SESSION.into());
        req.model = Some("opus".into());
        let invocation = ClaudeAdapter.invocation(&req, &[]);

        let resume = invocation
            .args
            .iter()
            .position(|arg| arg == "--resume")
            .expect("a follow-up turn must resume the session");
        assert_eq!(invocation.args[resume + 1], SESSION);

        let model = invocation.args.iter().position(|a| a == "--model").unwrap();
        assert_eq!(invocation.args[model + 1], "opus");
    }
}
