use serde_json::Value;

use crate::engine::{
    system_prompt, Block, CliAdapter, Invocation, Loaded, ParseState, RunRequest, StreamEvent,
    Usage,
};

/// Namespaced so a user's own agent of the same name is never the one that runs.
pub const CHAT_AGENT: &str = "starlux-chat";
const TITLE_AGENT: &str = "starlux-title";

/// opencode's own name for its fetcher, and the only tool Starlux grants that
/// it has: there is no search tool to allow, so `webSearch` has nothing to
/// translate to here. Named exactly rather than by category, so an allowlist
/// cannot grow with the provider.
const WEB_TOOL: &str = "webfetch";

/// Injected rather than written to disk: the user's own `opencode.json` is
/// theirs, and a launcher has no business editing it. This is opencode's
/// answer to Claude's `--agents` argv JSON.
const CONFIG_ENV: &str = "OPENCODE_CONFIG_CONTENT";

/// What `acp::run` spawns. The same agent the CLI path builds, plus the model
/// at the root of the config: `opencode acp` takes neither in argv, and picks
/// the agent up only when it is asked for over the wire by name.
pub fn acp_invocation(req: &RunRequest) -> Invocation {
    let mut config = serde_json::json!({ "agent": chat_agent(req) });
    if let Some(model) = &req.model {
        config["model"] = model.clone().into();
    }

    Invocation {
        program: "opencode".into(),
        args: vec!["acp".into()],
        // The protocol is the conversation, so stdin stays open — the hang that
        // forces `run`'s prompt into argv is a `run` problem, not opencode's.
        stdin: None,
        cwd: req.agent_dir.clone(),
        env: vec![(CONFIG_ENV.into(), config.to_string())],
    }
}

pub struct OpencodeAdapter;

impl CliAdapter for OpencodeAdapter {
    fn invocation(&self, req: &RunRequest, files: &[Loaded]) -> Invocation {
        let mut args = vec!["run".into(), "--format".into(), "json".into()];

        if let Some(model) = &req.model {
            args.push("-m".into());
            args.push(model.clone());
        }
        if let Some(session) = &req.session_id {
            args.push("-s".into());
            args.push(session.clone());
        }
        // opencode calls a thinking level a variant, and which ones exist is
        // the model's own answer — `providers` reads them from the same binary
        // rather than offering a ladder every model is assumed to have.
        if let Some(effort) = &req.effort {
            args.push("--variant".into());
            args.push(effort.clone());
        }

        args.push("--agent".into());
        args.push(CHAT_AGENT.into());

        for file in files {
            args.push("-f".into());
            args.push(file.path.to_string_lossy().into_owned());
        }

        // `-f` is a yargs array flag and swallows the positional message, which
        // then reads as a filename: `File not found: What colour is this?`.
        args.push("--".into());
        args.push(req.prompt.clone());

        Invocation {
            program: "opencode".into(),
            args,
            // opencode 1.18 blocks forever on an open stdin and never reaches
            // the model, so the prompt goes in argv instead. Still an argv
            // array, never a shell string.
            stdin: None,
            cwd: req.agent_dir.clone(),
            env: vec![(CONFIG_ENV.into(), config(chat_agent(req)))],
        }
    }

    /// Named on the conversation's own model. opencode otherwise resolves one
    /// itself — a cheaper model from the provider, else the configured default —
    /// and that default is reached whether or not the account can still spend
    /// on it, so a conversation answered on a free model went unnamed against a
    /// paid one. The model the user picked is the only one known to work.
    fn title_invocation(&self, question: &str, model: Option<&str>) -> Invocation {
        let mut args = vec!["run".into(), "--format".into(), "json".into()];

        if let Some(model) = model {
            args.push("-m".into());
            args.push(model.to_owned());
        }

        args.push("--agent".into());
        args.push(TITLE_AGENT.into());
        args.push("--".into());
        args.push(question.to_owned());

        Invocation {
            program: "opencode".into(),
            args,
            stdin: None,
            cwd: None,
            env: vec![(CONFIG_ENV.into(), config(title_agent()))],
        }
    }

    fn parse_line(&self, line: &str, state: &mut ParseState, req: &RunRequest) -> Vec<StreamEvent> {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            return Vec::new();
        };

        let mut events = Vec::new();
        let run_id = req.run_id.clone();

        if state.session_id.is_none() {
            if let Some(session) = value.get("sessionID").and_then(Value::as_str) {
                state.session_id = Some(session.to_owned());
                events.push(StreamEvent::Meta {
                    run_id: run_id.clone(),
                    session_id: state.session_id.clone(),
                    model: req.model.clone(),
                });
            }
        }

        match value.get("type").and_then(Value::as_str) {
            Some("text") => {
                if let Some(delta) = rewrite(&value, state) {
                    events.push(StreamEvent::Chunk { run_id, delta });
                }
            }
            // One per step, and a turn that calls a tool has several. The last
            // one is not announced, so they are totalled and read at the end.
            Some("step_finish") => add_usage(&value, state),
            Some("error") => {
                state.ended = true;
                events.push(StreamEvent::Error {
                    run_id,
                    message: message(&value).unwrap_or("the provider reported an error".into()),
                    stderr_tail: String::new(),
                });
            }
            _ => {}
        }

        events
    }

    /// opencode never says it has finished — it stops. A run that produced
    /// nothing is left to `cli`, which knows the exit status and can say so.
    fn finish(&self, state: &mut ParseState, req: &RunRequest) -> Vec<StreamEvent> {
        if state.ended || state.text.is_empty() {
            return Vec::new();
        }
        state.ended = true;
        vec![StreamEvent::End {
            run_id: req.run_id.clone(),
            text: state.text.clone(),
            session_id: state.session_id.clone(),
            usage: state.usage.clone(),
        }]
    }

    fn title_text(&self, stdout: &str) -> String {
        stdout
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .filter(|value| value.get("type").and_then(Value::as_str) == Some("text"))
            .filter_map(|value| Some(text_of(&value)?.to_owned()))
            .collect::<Vec<_>>()
            .join("")
    }
}

/// A text block arrives whole and can arrive again once it has grown, so what
/// is new is whatever the block gained since it was last seen.
fn rewrite(value: &Value, state: &mut ParseState) -> Option<String> {
    let part = value.get("part")?;
    let id = part.get("id")?.as_str()?;
    let text = text_of(value)?;

    let at = match &state.block {
        Some(block) if block.id == id => block.at,
        _ => {
            let at = state.text.len();
            state.block = Some(Block {
                id: id.to_owned(),
                at,
            });
            at
        }
    };

    let already = state.text.len() - at;
    let delta = text.get(already..).unwrap_or_default().to_owned();
    state.text.truncate(at);
    state.text.push_str(text);
    (!delta.is_empty()).then_some(delta)
}

fn text_of(value: &Value) -> Option<&str> {
    value.get("part")?.get("text")?.as_str()
}

fn message(value: &Value) -> Option<String> {
    let error = value.get("error")?;
    let named = error
        .get("data")
        .and_then(|data| data.get("message"))
        .or_else(|| error.get("message"))?
        .as_str()?;
    Some(named.to_owned())
}

fn add_usage(value: &Value, state: &mut ParseState) {
    let Some(part) = value.get("part") else {
        return;
    };
    let tokens = part.get("tokens");
    let count = |name: &str| {
        tokens
            .and_then(|t| t.get(name))
            .and_then(Value::as_u64)
            .unwrap_or(0)
    };

    let running = state.usage.get_or_insert(Usage {
        input_tokens: 0,
        output_tokens: 0,
        cost_usd: None,
        // opencode reports neither a window size nor how full it is, and
        // inferring one from the model name would be a guess dressed as
        // arithmetic.
        context: None,
    });
    running.input_tokens += count("input");
    running.output_tokens += count("output");
    if let Some(cost) = part.get("cost").and_then(Value::as_f64) {
        running.cost_usd = Some(running.cost_usd.unwrap_or(0.0) + cost);
    }
}

fn config(agent: Value) -> String {
    serde_json::json!({ "agent": agent }).to_string()
}

/// Agent mode relaxes the permission map rather than passing a folder flag:
/// opencode takes its directory from the process's own, which `cli` sets.
///
/// Its prompt is left alone there, so opencode's tool instructions survive —
/// which does mean an agent-mode run is not told Starlux's rendering rules.
fn chat_agent(req: &RunRequest) -> Value {
    let mut permission = serde_json::Map::new();
    permission.insert(
        "*".into(),
        if req.agent_dir.is_some() {
            "allow".into()
        } else {
            "deny".into()
        },
    );
    if req.tools.web_fetch {
        permission.insert(WEB_TOOL.into(), "allow".into());
    }

    let mut agent = serde_json::json!({
        "description": "Answers questions from the Starlux bar",
        "mode": "primary",
        "permission": permission,
    });
    if req.agent_dir.is_none() {
        agent["prompt"] = system_prompt::chat().into();
    }
    serde_json::json!({ CHAT_AGENT: agent })
}

fn title_agent() -> Value {
    serde_json::json!({
        TITLE_AGENT: {
            "description": "Names a Starlux conversation",
            "mode": "primary",
            "prompt": system_prompt::title(),
            "permission": { "*": "deny" },
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::Tools;
    use std::path::PathBuf;

    /// A real turn, captured from opencode 1.18.21. The JSON shape drifted once
    /// already — `text` moved under `part` and the step events were renamed —
    /// so an auto-update that moves it again fails here rather than going quiet.
    const FIXTURE: &str = include_str!("../../../tests/fixtures/opencode-stream.ndjson");

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
        }
    }

    fn drain(lines: &[&str]) -> (Vec<StreamEvent>, ParseState) {
        let adapter = OpencodeAdapter;
        let req = request();
        let mut state = ParseState::default();
        let mut events: Vec<_> = lines
            .iter()
            .flat_map(|line| adapter.parse_line(line, &mut state, &req))
            .collect();
        events.extend(adapter.finish(&mut state, &req));
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

    #[allow(clippy::type_complexity)]
    fn ending(events: &[StreamEvent]) -> Option<(&String, &Option<String>, &Option<Usage>)> {
        events.iter().find_map(|event| match event {
            StreamEvent::End {
                text,
                session_id,
                usage,
                ..
            } => Some((text, session_id, usage)),
            _ => None,
        })
    }

    fn loaded(name: &str) -> Loaded {
        Loaded {
            path: PathBuf::from("/tmp").join(name),
            name: name.to_owned(),
            mime: "image/png".into(),
            data: Vec::new(),
        }
    }

    fn env_of(invocation: &Invocation) -> Value {
        let (_, config) = invocation
            .env
            .iter()
            .find(|(key, _)| key == CONFIG_ENV)
            .expect("every run carries the agent that bounds it");
        serde_json::from_str(config).unwrap()
    }

    #[test]
    fn reads_a_captured_turn_end_to_end() {
        let lines: Vec<&str> = FIXTURE.lines().collect();
        let (events, state) = drain(&lines);

        assert!(chunks(&events).starts_with("**example.com h1"));
        let (text, session, usage) = ending(&events).expect("a run that answered must end");
        assert_eq!(text, &state.text);
        assert_eq!(session.as_deref(), Some("ses_fcf0f9b57ffeWHjpMARwLC42kC"));

        // Two steps, so a turn that called a tool reports the sum rather than
        // whichever half happened to come last.
        let usage = usage.as_ref().expect("a finished turn reports its tokens");
        assert!(usage.input_tokens > 0 && usage.output_tokens > 0);
        assert_eq!(usage.context, None);
    }

    /// It never says it has finished, it stops — so the turn is ended by the
    /// stream closing rather than by anything in it.
    #[test]
    fn nothing_in_the_stream_ends_the_turn() {
        let lines: Vec<&str> = FIXTURE.lines().collect();
        let adapter = OpencodeAdapter;
        let req = request();
        let mut state = ParseState::default();
        for line in &lines {
            adapter.parse_line(line, &mut state, &req);
        }
        assert!(!state.ended);
        assert_eq!(adapter.finish(&mut state, &req).len(), 1);
        assert!(state.ended);
    }

    /// A run that died before saying anything is left to `cli`, which knows the
    /// exit status. Ending it here would report silence as a successful answer.
    #[test]
    fn a_run_that_said_nothing_does_not_end_itself() {
        let (events, state) = drain(&[]);
        assert!(events.is_empty());
        assert!(!state.ended);
    }

    #[test]
    fn a_block_that_arrives_again_having_grown_is_not_repeated() {
        let block = |text: &str| {
            format!(r#"{{"type":"text","sessionID":"s1","part":{{"id":"p1","text":"{text}"}}}}"#)
        };
        let (events, state) = drain(&[&block("Hello"), &block("Hello, world")]);
        assert_eq!(chunks(&events), "Hello, world");
        assert_eq!(state.text, "Hello, world");
    }

    #[test]
    fn a_second_block_follows_the_first_rather_than_replacing_it() {
        let (events, state) = drain(&[
            r#"{"type":"text","sessionID":"s1","part":{"id":"p1","text":"one "}}"#,
            r#"{"type":"text","sessionID":"s1","part":{"id":"p2","text":"two"}}"#,
        ]);
        assert_eq!(chunks(&events), "one two");
        assert_eq!(state.text, "one two");
    }

    #[test]
    fn surfaces_the_error_the_provider_reported() {
        let (events, state) = drain(&[
            r#"{"type":"error","sessionID":"s1","error":{"name":"APIError","data":{"message":"Insufficient balance."}}}"#,
        ]);
        assert!(state.ended);
        assert!(matches!(
            events.last(),
            Some(StreamEvent::Error { message, .. }) if message == "Insufficient balance."
        ));
    }

    #[test]
    fn survives_lines_it_cannot_read() {
        let (events, state) = drain(&["", "not json", r#"{"type":"text","part":{}}"#]);
        assert!(chunks(&events).is_empty());
        assert!(!state.ended);
    }

    /// It blocks forever on an open stdin and never reaches the model, so the
    /// prompt goes in argv — still an array, never a shell string.
    #[test]
    fn the_prompt_goes_in_argv_because_stdin_would_hang() {
        let invocation = OpencodeAdapter.invocation(&request(), &[]);
        assert_eq!(invocation.program, "opencode");
        assert_eq!(invocation.stdin, None);
        assert_eq!(invocation.args.last().unwrap(), "what colour is this?");
    }

    /// `-f` is a yargs array flag: without the separator it swallows the
    /// message and reports it as a filename that does not exist.
    #[test]
    fn attachments_are_separated_from_the_message() {
        let png = loaded("blue.png");
        let invocation = OpencodeAdapter.invocation(&request(), std::slice::from_ref(&png));
        let file = invocation.args.iter().position(|a| a == "-f").unwrap();
        let stop = invocation.args.iter().position(|a| a == "--").unwrap();
        assert!(file < stop);
        // Spelled the way the platform spells it: a path built here with `join`
        // wears a backslash on Windows, and the CLI wants what the OS gave us.
        assert_eq!(invocation.args[file + 1], png.path.to_string_lossy());
        assert_eq!(invocation.args[stop + 1], "what colour is this?");
    }

    fn permission(invocation: &Invocation, agent: &str) -> Value {
        env_of(invocation)["agent"][agent]["permission"].clone()
    }

    #[test]
    fn chat_only_denies_every_tool_it_has() {
        let invocation = OpencodeAdapter.invocation(&request(), &[]);
        assert_eq!(
            permission(&invocation, CHAT_AGENT),
            serde_json::json!({ "*": "deny" })
        );
        assert_eq!(invocation.cwd, None);
        assert!(env_of(&invocation)["agent"][CHAT_AGENT]["prompt"]
            .as_str()
            .unwrap()
            .starts_with("You are Starlux"));
    }

    #[test]
    fn the_fetch_grant_opens_the_fetcher_and_nothing_else() {
        let mut req = request();
        req.tools.web_fetch = true;
        let invocation = OpencodeAdapter.invocation(&req, &[]);
        assert_eq!(
            permission(&invocation, CHAT_AGENT),
            serde_json::json!({ "*": "deny", "webfetch": "allow" })
        );
        assert_eq!(invocation.cwd, None);
    }

    /// opencode has no search tool, so the app-wide grant has nothing to
    /// translate to here and must not open the fetcher in its place.
    #[test]
    fn the_search_grant_opens_nothing_where_there_is_no_search_tool() {
        let mut req = request();
        req.tools.web_search = true;
        let invocation = OpencodeAdapter.invocation(&req, &[]);
        assert_eq!(
            permission(&invocation, CHAT_AGENT),
            serde_json::json!({ "*": "deny" })
        );
    }

    /// opencode takes its directory from the process's own, so agent mode is a
    /// relaxed permission map and a cwd rather than a folder flag.
    #[test]
    fn agent_mode_relaxes_the_map_and_keeps_the_provider_prompt() {
        let mut req = request();
        req.agent_dir = Some(PathBuf::from("/tmp/project"));
        let invocation = OpencodeAdapter.invocation(&req, &[]);

        assert_eq!(
            permission(&invocation, CHAT_AGENT),
            serde_json::json!({ "*": "allow" })
        );
        assert_eq!(invocation.cwd, Some(PathBuf::from("/tmp/project")));
        assert!(env_of(&invocation)["agent"][CHAT_AGENT]["prompt"].is_null());
    }

    /// The ACP run is bounded by the same agent as the CLI run — it just has to
    /// be asked for by name over the wire, because `opencode acp` takes no
    /// `--agent` flag and ignores `default_agent`.
    #[test]
    fn the_acp_run_carries_the_same_agent_and_denies_the_same_tools() {
        let invocation = acp_invocation(&request());
        assert_eq!(invocation.args, ["acp"]);
        assert_eq!(
            permission(&invocation, CHAT_AGENT),
            serde_json::json!({ "*": "deny" })
        );
        assert!(env_of(&invocation)["agent"][CHAT_AGENT]["prompt"]
            .as_str()
            .unwrap()
            .starts_with("You are Starlux"));
    }

    /// It takes no `-m` either, so the model rides at the root of the config.
    #[test]
    fn the_acp_run_names_its_model_in_the_config() {
        assert_eq!(
            env_of(&acp_invocation(&request()))["model"],
            "opencode/hy3-free"
        );
    }

    #[test]
    fn the_acp_run_grants_the_fetcher_when_the_app_does() {
        let mut req = request();
        req.tools.web_fetch = true;
        assert_eq!(
            permission(&acp_invocation(&req), CHAT_AGENT),
            serde_json::json!({ "*": "deny", "webfetch": "allow" })
        );
    }

    /// Agent mode is a cwd here as it is there: opencode takes its directory
    /// from the process's own either way.
    #[test]
    fn the_acp_run_takes_its_folder_as_a_directory() {
        let mut req = request();
        req.agent_dir = Some(PathBuf::from("/tmp/project"));
        let invocation = acp_invocation(&req);
        assert_eq!(invocation.cwd, Some(PathBuf::from("/tmp/project")));
        assert_eq!(
            permission(&invocation, CHAT_AGENT),
            serde_json::json!({ "*": "allow" })
        );
    }

    #[test]
    fn naming_a_conversation_is_toolless_and_starts_no_session() {
        let invocation = OpencodeAdapter.title_invocation("what is a spectral class?", None);
        assert_eq!(
            permission(&invocation, TITLE_AGENT),
            serde_json::json!({ "*": "deny" })
        );
        assert!(!invocation.args.iter().any(|arg| arg == "-s"));
        assert_eq!(invocation.args.last().unwrap(), "what is a spectral class?");
    }

    /// Naming nothing left opencode to resolve a model itself, and it resolved
    /// one the account could not spend on: a conversation answered on a free
    /// model failed to be named against a paid default, with `Insufficient
    /// balance`. The model the user picked is the one known to work.
    #[test]
    fn a_conversation_is_named_on_the_model_it_is_answered_on() {
        let invocation = OpencodeAdapter.title_invocation("what is a spectral class?", None);
        assert!(!invocation.args.iter().any(|arg| arg == "-m"));

        let invocation = OpencodeAdapter
            .title_invocation("what is a spectral class?", Some("opencode/hy3-free"));
        let at = invocation
            .args
            .iter()
            .position(|arg| arg == "-m")
            .expect("the naming run must not pick its own model");
        assert_eq!(invocation.args[at + 1], "opencode/hy3-free");
        assert_eq!(invocation.args.last().unwrap(), "what is a spectral class?");
    }

    /// The naming run only speaks JSON, so the title has to be unwrapped before
    /// `title::clean` sees it — otherwise the conversation is named `{"type":`.
    #[test]
    fn the_title_is_read_out_of_the_json_it_is_wrapped_in() {
        let printed = concat!(
            r#"{"type":"step_start","part":{"id":"p0"}}"#,
            "\n",
            r#"{"type":"text","part":{"id":"p1","text":"Spectral classes"}}"#,
            "\n",
        );
        assert_eq!(OpencodeAdapter.title_text(printed), "Spectral classes");
    }

    #[test]
    fn no_level_chosen_leaves_the_variant_flag_off() {
        let invocation = OpencodeAdapter.invocation(&request(), &[]);
        assert!(!invocation.args.iter().any(|arg| arg == "--variant"));
    }

    /// opencode spells a thinking level `--variant`, and the names are the
    /// model's own: `max` exists on `glm-5.3` and `thinking` on `minimax-m3`.
    #[test]
    fn carries_the_chosen_thinking_level_as_a_variant() {
        let mut req = request();
        req.effort = Some("max".into());
        let invocation = OpencodeAdapter.invocation(&req, &[]);
        let at = invocation
            .args
            .iter()
            .position(|a| a == "--variant")
            .unwrap();
        assert_eq!(invocation.args[at + 1], "max");
        // Still ahead of the separator, or it reads as part of the message.
        let stop = invocation.args.iter().position(|a| a == "--").unwrap();
        assert!(at < stop);
    }

    #[test]
    fn resumes_the_session_and_pins_the_model() {
        let mut req = request();
        req.session_id = Some("ses_42".into());
        let invocation = OpencodeAdapter.invocation(&req, &[]);

        let session = invocation.args.iter().position(|a| a == "-s").unwrap();
        assert_eq!(invocation.args[session + 1], "ses_42");
        let model = invocation.args.iter().position(|a| a == "-m").unwrap();
        assert_eq!(invocation.args[model + 1], "opencode/hy3-free");
    }
}
