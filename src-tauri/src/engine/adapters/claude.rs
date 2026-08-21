use serde_json::Value;

use crate::engine::{CliAdapter, Invocation, ParseState, RunRequest, StreamEvent, Usage};

pub struct ClaudeAdapter;

impl CliAdapter for ClaudeAdapter {
    fn invocation(&self, req: &RunRequest) -> Invocation {
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

        // Chat-only unless the conversation opted into agent mode. `--bare` is
        // deliberately not used: it forces ANTHROPIC_API_KEY auth, which would
        // bypass the subscription this bridge exists to use.
        if req.agent_dir.is_none() {
            args.push("--allowed-tools".into());
            args.push(String::new());
        }

        Invocation {
            program: "claude".into(),
            args,
            stdin: Some(req.prompt.clone()),
            cwd: req.agent_dir.clone(),
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
                        usage: usage(&value),
                    });
                }
            }
            _ => {}
        }

        events
    }
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

fn usage(value: &Value) -> Option<Usage> {
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
            provider_id: "claude-cli".into(),
            prompt: "count to 3".into(),
            session_id: None,
            model: None,
            agent_dir: None,
        }
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
    fn chat_only_disables_tools_and_inherits_no_directory() {
        let invocation = ClaudeAdapter.invocation(&request());
        assert_eq!(invocation.program, "claude");
        assert_eq!(invocation.stdin.as_deref(), Some("count to 3"));
        assert_eq!(invocation.cwd, None);

        let tools = invocation
            .args
            .iter()
            .position(|arg| arg == "--allowed-tools")
            .expect("chat-only runs must disable tools");
        assert_eq!(invocation.args[tools + 1], "");
        assert!(!invocation.args.iter().any(|arg| arg == "--bare"));
    }

    #[test]
    fn agent_mode_enables_tools_pinned_to_a_directory() {
        let mut req = request();
        req.agent_dir = Some(PathBuf::from("/tmp/project"));
        let invocation = ClaudeAdapter.invocation(&req);

        assert_eq!(invocation.cwd, Some(PathBuf::from("/tmp/project")));
        assert!(!invocation.args.iter().any(|arg| arg == "--allowed-tools"));
    }

    #[test]
    fn resumes_the_provider_session_for_multi_turn() {
        let mut req = request();
        req.session_id = Some(SESSION.into());
        req.model = Some("opus".into());
        let invocation = ClaudeAdapter.invocation(&req);

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
