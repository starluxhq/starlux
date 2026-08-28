//! Prints the argv, env and stdin an adapter builds, so a provider can be
//! exercised with exactly what the app would have sent it.
//!
//! This is what makes the canary check reproducible: run it, feed the result to
//! the CLI, and ask the model to read a file it must not be able to reach. A
//! chat-only grant that has quietly stopped being one shows up there and
//! nowhere else — no unit test can see past the argv it asserts.
//!
//! A provider driven over ACP prints what `acp::run` spawns, which is what a
//! real turn uses; its `run` argv is only ever the naming call now.
//!
//! ```sh
//! cargo run --example invocation -- opencode-cli "what colour is this?" blue.png
//! SEARCH=1 MODEL=opus cargo run --example invocation -- claude-cli "what shipped?"
//! FETCH=1 cargo run --example invocation -- gemini-cli "what does example.com say?"
//! TITLE=1 cargo run --example invocation -- claude-cli "how do pulsars work?"
//! ```
use starlux_lib::engine::{adapters, file_name, mime_of, providers, Loaded, RunRequest, Tools};

fn main() {
    let mut args = std::env::args().skip(1);
    let provider = args.next().expect("provider id");
    let prompt = args.next().expect("prompt");
    let req = RunRequest {
        run_id: "run-1".into(),
        conversation_id: "conv-1".into(),
        provider_id: provider.clone(),
        prompt,
        session_id: None,
        model: std::env::var("MODEL").ok(),
        effort: std::env::var("EFFORT").ok(),
        agent_dir: None,
        tools: Tools {
            web_search: std::env::var("SEARCH").is_ok(),
            web_fetch: std::env::var("FETCH").is_ok(),
        },
        attachments: args.map(Into::into).collect(),
    };
    let files: Vec<Loaded> = req
        .attachments
        .iter()
        .map(|path| Loaded {
            name: file_name(path),
            mime: mime_of(path).to_owned(),
            data: std::fs::read(path).unwrap(),
            path: path.clone(),
        })
        .collect();

    let adapter = adapters::for_provider(&provider).unwrap();
    if std::env::var("TITLE").is_ok() {
        adapter.prepare_title().unwrap();
    } else {
        adapter.prepare(&req, &files).unwrap();
    }
    let invocation = if std::env::var("TITLE").is_ok() {
        adapter.title_invocation(&req.prompt)
    } else if providers::speaks_acp(&provider) {
        adapters::opencode::acp_invocation(&req)
    } else {
        adapter.invocation(&req, &files)
    };
    println!(
        "{}",
        serde_json::json!({
            "program": invocation.program,
            "args": invocation.args,
            "env": invocation.env,
            "stdin": invocation.stdin,
        })
    );
}
