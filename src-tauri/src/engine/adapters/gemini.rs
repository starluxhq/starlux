use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::engine::{
    data_dir, system_prompt, CliAdapter, Invocation, Loaded, ParseState, RunRequest, StreamEvent,
    Usage,
};

/// gemini's own names for the two tools that reach the network, and for the two
/// that read a file. Named exactly rather than by category, so an allowlist
/// cannot grow with the provider.
const WEB_TOOLS: [&str; 2] = ["google_web_search", "web_fetch"];
const READ_TOOLS: [&str; 2] = ["read_file", "read_many_files"];

/// Within a tier, higher wins. Denying everything sits below the two narrow
/// allowances so either can lift a single tool out of the deny.
const DENY_PRIORITY: u32 = 900;
const ALLOW_PRIORITY: u32 = 950;

/// Policies written before every run, never left to be found. The whole grant
/// lives in this file, so a stale one is a grant nobody made.
const TITLE_POLICY: &str = "title";

pub struct GeminiAdapter;

impl CliAdapter for GeminiAdapter {
    /// Unlike the other two, gemini reads files by default when run headless:
    /// with no policy and the default approval mode it opened a canary file and
    /// printed the contents. There is no argv-only way to stop that —
    /// `--allowed-tools` is deprecated and is an approve-without-confirmation
    /// list, not a restriction — so the policy file is not optional, and this is
    /// what writes it.
    fn prepare(&self, req: &RunRequest, files: &[Loaded]) -> Result<(), String> {
        write_policy(&policy_path(&req.run_id), &policy(req.web, files))
    }

    fn prepare_title(&self) -> Result<(), String> {
        write_policy(&policy_path(TITLE_POLICY), &policy(false, &[]))
    }

    fn invocation(&self, req: &RunRequest, files: &[Loaded]) -> Invocation {
        let mut args = vec![
            "-o".into(),
            "stream-json".into(),
            // It refuses to run headless without this, exiting 55 with "not
            // running in a trusted directory". Not a security decision once the
            // policy denies everything the run was not granted.
            "--skip-trust".into(),
            "--policy".into(),
            policy_path(&req.run_id).to_string_lossy().into_owned(),
        ];

        if let Some(model) = &req.model {
            args.push("--model".into());
            args.push(model.clone());
        }
        if let Some(session) = &req.session_id {
            // Despite what `--help` says about indexes and "latest", this takes
            // the session id `init` reported. `--session-id` would refuse: that
            // one only ever starts a session.
            args.push("--resume".into());
            args.push(session.clone());
        }

        for dir in parents(files) {
            args.push("--include-directories".into());
            args.push(dir.to_string_lossy().into_owned());
        }

        args.push("-p".into());
        args.push(prompt(req, files));

        Invocation {
            program: "gemini".into(),
            args,
            // gemini appends `-p` to whatever is on stdin, so a prompt sent both
            // ways would arrive twice.
            stdin: None,
            cwd: req.agent_dir.clone(),
            env: Vec::new(),
        }
    }

    fn title_invocation(&self, question: &str) -> Invocation {
        Invocation {
            program: "gemini".into(),
            args: vec![
                "-o".into(),
                "text".into(),
                "--skip-trust".into(),
                "--policy".into(),
                policy_path(TITLE_POLICY).to_string_lossy().into_owned(),
                "-p".into(),
                format!("{}\n\n{question}", system_prompt::title()),
            ],
            stdin: None,
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

        if state.session_id.is_none() {
            if let Some(session) = value.get("session_id").and_then(Value::as_str) {
                state.session_id = Some(session.to_owned());
                // `auto` is the router's name for itself rather than a model,
                // so what was asked for is the truer answer.
                state.model = value
                    .get("model")
                    .and_then(Value::as_str)
                    .filter(|model| *model != "auto")
                    .map(str::to_owned)
                    .or_else(|| req.model.clone());
                events.push(StreamEvent::Meta {
                    run_id: run_id.clone(),
                    session_id: state.session_id.clone(),
                    model: state.model.clone(),
                });
            }
        }

        match value.get("type").and_then(Value::as_str) {
            // The user's own question comes back first; only the assistant's
            // half is the answer.
            Some("message") if value.get("role").and_then(Value::as_str) == Some("assistant") => {
                if let Some(text) = value.get("content").and_then(Value::as_str) {
                    let delta = value.get("delta").and_then(Value::as_bool).unwrap_or(false);
                    if delta {
                        state.saw_delta = true;
                    } else if state.saw_delta {
                        // A whole message after deltas is the same text again.
                        return events;
                    }
                    state.text.push_str(text);
                    events.push(StreamEvent::Chunk {
                        run_id,
                        delta: text.to_owned(),
                    });
                }
            }
            Some("result") => {
                state.ended = true;
                match failure(&value) {
                    Some(message) => events.push(StreamEvent::Error {
                        run_id,
                        message,
                        stderr_tail: String::new(),
                    }),
                    None => events.push(StreamEvent::End {
                        run_id,
                        text: state.text.clone(),
                        session_id: state.session_id.clone(),
                        usage: usage(&value),
                    }),
                }
            }
            _ => {}
        }

        events
    }
}

/// Attachments are `@<path>` references, which gemini expands out of the prompt
/// text. They go after the user's own words rather than into them, so an `@`
/// they typed is never mistaken for one of ours.
fn prompt(req: &RunRequest, files: &[Loaded]) -> String {
    let mut prompt = req.prompt.clone();
    for file in files {
        prompt.push_str("\n\n@");
        prompt.push_str(&file.path.to_string_lossy());
    }
    prompt
}

fn parents(files: &[Loaded]) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    for dir in files.iter().filter_map(|file| file.path.parent()) {
        if !dirs.iter().any(|seen| seen == dir) {
            dirs.push(dir.to_owned());
        }
    }
    dirs
}

fn policy_path(name: &str) -> PathBuf {
    data_dir()
        .join("gemini")
        .join(format!("{}.toml", name.replace(['/', '\\', '.'], "-")))
}

fn write_policy(path: &Path, body: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("could not prepare gemini's policy: {err}"))?;
    }
    std::fs::write(path, body).map_err(|err| format!("could not write gemini's policy: {err}"))
}

/// Deny everything, then lift out exactly what was granted.
///
/// An attachment is the one grant the user makes by name: `@path` is expanded
/// by a read tool, so the file cannot arrive while reading is denied. The
/// allowance is pinned to the paths they attached rather than to reading at
/// large, so attaching a screenshot does not also open the filesystem.
fn policy(web: bool, files: &[Loaded]) -> String {
    let mut rules =
        format!("[[rule]]\ntoolName = \"*\"\ndecision = \"deny\"\npriority = {DENY_PRIORITY}\n");

    if web {
        rules.push_str(&allow(&WEB_TOOLS, None));
    }
    if !files.is_empty() {
        let paths = files
            .iter()
            .map(|file| escape(&file.path.to_string_lossy()))
            .collect::<Vec<_>>()
            .join("|");
        rules.push_str(&allow(&READ_TOOLS, Some(&format!("\"({paths})\""))));
    }

    rules
}

fn allow(tools: &[&str], args_pattern: Option<&str>) -> String {
    let names = tools
        .iter()
        .map(|tool| format!("\"{tool}\""))
        .collect::<Vec<_>>()
        .join(", ");
    let mut rule = format!(
        "\n[[rule]]\ntoolName = [{names}]\ndecision = \"allow\"\npriority = {ALLOW_PRIORITY}\n"
    );
    if let Some(pattern) = args_pattern {
        // TOML literal strings take no escapes, which is what keeps a regex's
        // own backslashes intact.
        rule.push_str(&format!("argsPattern = '{pattern}'\n"));
    }
    rule
}

/// The pattern is matched against the tool call's arguments as JSON, so a path
/// has to arrive as a regex that means only itself.
fn escape(path: &str) -> String {
    path.chars()
        .flat_map(|c| {
            let escaped = matches!(
                c,
                '.' | '+' | '*' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|' | '\\'
            );
            escaped.then_some('\\').into_iter().chain(Some(c))
        })
        .collect()
}

fn failure(value: &Value) -> Option<String> {
    if value.get("status").and_then(Value::as_str) == Some("success") {
        return None;
    }
    Some(
        value
            .get("error")
            .and_then(|error| error.get("message"))
            .and_then(Value::as_str)
            .unwrap_or("the provider reported an error")
            .to_owned(),
    )
}

fn usage(value: &Value) -> Option<Usage> {
    let stats = value.get("stats")?;
    let count = |name| stats.get(name).and_then(Value::as_u64).unwrap_or(0);
    Some(Usage {
        input_tokens: count("input_tokens"),
        output_tokens: count("output_tokens"),
        cost_usd: None,
        // It reports what a turn spent but never how large the window is, and
        // inferring one from the model name would be a guess dressed as
        // arithmetic.
        context: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SESSION: &str = "0f6fbcee-2708-487a-8448-460146eb9cd6";

    fn request() -> RunRequest {
        RunRequest {
            run_id: "run-1".into(),
            conversation_id: "conv-1".into(),
            provider_id: "gemini-cli".into(),
            prompt: "what colour is this?".into(),
            session_id: None,
            model: Some("gemini-3.5-flash".into()),
            agent_dir: None,
            web: false,
            attachments: Vec::new(),
        }
    }

    fn loaded(path: &str) -> Loaded {
        let path = PathBuf::from(path);
        Loaded {
            name: crate::engine::file_name(&path),
            mime: crate::engine::mime_of(&path).to_owned(),
            path,
            data: Vec::new(),
        }
    }

    fn drain(lines: &[&str]) -> (Vec<StreamEvent>, ParseState) {
        let adapter = GeminiAdapter;
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
    fn streams_the_assistant_half_and_ignores_the_question_coming_back() {
        let (events, state) = drain(&[
            &format!(r#"{{"type":"init","session_id":"{SESSION}","model":"auto"}}"#),
            r#"{"type":"message","role":"user","content":"what colour is this?"}"#,
            r#"{"type":"message","role":"assistant","content":"Mercury, Venus","delta":true}"#,
            r#"{"type":"message","role":"assistant","content":", Earth","delta":true}"#,
            r#"{"type":"result","status":"success","stats":{"input_tokens":9936,"output_tokens":28}}"#,
        ]);

        assert_eq!(chunks(&events), "Mercury, Venus, Earth");
        assert!(state.ended);

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
            .expect("the result line ends the turn");
        assert_eq!(end.0, "Mercury, Venus, Earth");
        assert_eq!(end.1.as_deref(), Some(SESSION));
        let usage = end.2.as_ref().unwrap();
        assert_eq!((usage.input_tokens, usage.output_tokens), (9936, 28));
        assert_eq!(usage.context, None);
    }

    /// `auto` is the router naming itself, not a model. Reporting it would put
    /// the word "auto" where the picker shows what answered.
    #[test]
    fn the_routers_own_name_is_not_reported_as_a_model() {
        let (events, _) = drain(&[&format!(
            r#"{{"type":"init","session_id":"{SESSION}","model":"auto"}}"#
        )]);
        assert!(matches!(
            events.first(),
            Some(StreamEvent::Meta { model: Some(model), .. }) if model == "gemini-3.5-flash"
        ));

        let (events, _) = drain(&[&format!(
            r#"{{"type":"init","session_id":"{SESSION}","model":"gemini-3.1-flash-lite"}}"#
        )]);
        assert!(matches!(
            events.first(),
            Some(StreamEvent::Meta { model: Some(model), .. }) if model == "gemini-3.1-flash-lite"
        ));
    }

    #[test]
    fn a_failed_result_is_the_error_it_names() {
        let (events, state) = drain(&[
            r#"{"type":"result","status":"error","error":{"type":"unknown","message":"[API Error: models/x is not found]"},"stats":{}}"#,
        ]);
        assert!(state.ended);
        assert!(matches!(
            events.last(),
            Some(StreamEvent::Error { message, .. }) if message.contains("is not found")
        ));
    }

    #[test]
    fn survives_lines_it_cannot_read() {
        let (events, state) = drain(&["", "not json", r#"{"type":"message","role":"assistant"}"#]);
        assert!(chunks(&events).is_empty());
        assert!(!state.ended);
    }

    /// It reads files by default when run headless — with no policy it opened a
    /// canary and printed the contents — so a run without this flag is a run
    /// with a filesystem.
    #[test]
    fn every_run_is_pointed_at_a_policy() {
        for invocation in [
            GeminiAdapter.invocation(&request(), &[]),
            GeminiAdapter.title_invocation("how do pulsars work?"),
        ] {
            let at = invocation
                .args
                .iter()
                .position(|arg| arg == "--policy")
                .expect("gemini must never run unbounded");
            assert!(invocation.args[at + 1].ends_with(".toml"));
            assert!(invocation.args.iter().any(|arg| arg == "--skip-trust"));
        }
    }

    #[test]
    fn chat_only_denies_every_tool_and_lifts_none_of_them() {
        let policy = policy(false, &[]);
        assert!(policy.contains(r#"toolName = "*""#));
        assert!(policy.contains(r#"decision = "deny""#));
        assert!(!policy.contains(r#"decision = "allow""#));
    }

    #[test]
    fn the_web_grant_lifts_the_two_web_tools_and_nothing_else() {
        let policy = policy(true, &[]);
        assert!(policy.contains(r#"toolName = ["google_web_search", "web_fetch"]"#));
        assert!(!policy.contains("read_file"));
        // The deny stays: it is what everything not named falls through to.
        assert!(policy.contains(r#"toolName = "*""#));
    }

    /// `@path` is expanded by a read tool, so a file cannot arrive while reading
    /// is denied. The allowance is pinned to the paths the user attached, so
    /// attaching a screenshot does not also open the filesystem.
    #[test]
    fn an_attachment_lifts_reading_only_for_its_own_path() {
        let policy = policy(false, &[loaded("/home/a/blue.png")]);
        assert!(policy.contains(r#"toolName = ["read_file", "read_many_files"]"#));
        assert!(policy.contains(r#"argsPattern = '"(/home/a/blue\.png)"'"#));
    }

    #[test]
    fn several_attachments_share_one_allowance() {
        let policy = policy(false, &[loaded("/a/one.png"), loaded("/b/two.md")]);
        assert!(policy.contains(r#"'"(/a/one\.png|/b/two\.md)"'"#));
        assert_eq!(policy.matches("read_file").count(), 1);
    }

    /// A path is a regex here, so anything in it that means something else has
    /// to be made to mean only itself.
    #[test]
    fn a_path_full_of_regex_becomes_a_pattern_that_means_itself() {
        assert_eq!(escape("/a/b (1).png"), r"/a/b \(1\)\.png");
        assert_eq!(escape(r"C:\Users\a.png"), r"C:\\Users\\a\.png");
    }

    /// A reference is appended, never interpolated, so an `@` the user typed is
    /// never mistaken for one of ours.
    #[test]
    fn attachments_are_named_after_the_question_not_inside_it() {
        let mut req = request();
        req.prompt = "what does @everyone mean here?".into();
        let files = [loaded("/home/a/blue.png")];
        let invocation = GeminiAdapter.invocation(&req, &files);

        let at = invocation.args.iter().position(|arg| arg == "-p").unwrap();
        assert_eq!(
            invocation.args[at + 1],
            "what does @everyone mean here?\n\n@/home/a/blue.png"
        );

        let dir = invocation
            .args
            .iter()
            .position(|arg| arg == "--include-directories")
            .expect("a file outside the run's own folder has to be reachable");
        assert_eq!(invocation.args[dir + 1], "/home/a");
    }

    /// `--help` describes `--resume` as taking "latest" or an index, but the
    /// CLI itself says to use it for a session id — and does.
    #[test]
    fn a_follow_up_resumes_by_session_id() {
        let mut req = request();
        req.session_id = Some(SESSION.into());
        let invocation = GeminiAdapter.invocation(&req, &[]);

        let at = invocation
            .args
            .iter()
            .position(|arg| arg == "--resume")
            .unwrap();
        assert_eq!(invocation.args[at + 1], SESSION);
        assert!(!invocation.args.iter().any(|arg| arg == "--session-id"));
    }

    /// The prompt is in argv; gemini appends `-p` to whatever is on stdin, so
    /// sending it both ways would ask the question twice.
    #[test]
    fn the_question_is_asked_once() {
        let invocation = GeminiAdapter.invocation(&request(), &[]);
        assert_eq!(invocation.stdin, None);
        assert_eq!(invocation.args.iter().filter(|arg| *arg == "-p").count(), 1);
    }

    /// A run id crosses IPC from a window, and it names a file.
    #[test]
    fn a_run_id_cannot_write_a_policy_outside_the_directory_meant_for_them() {
        let path = policy_path("../../etc/passwd");
        assert_eq!(path.parent(), Some(data_dir().join("gemini").as_path()));
        assert_eq!(path.file_name().unwrap(), "------etc-passwd.toml");
    }
}
