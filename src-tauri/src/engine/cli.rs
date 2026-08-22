use std::collections::{HashMap, VecDeque};
use std::process::Stdio;
use std::sync::{Arc, Mutex};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::Command;
use tokio::sync::oneshot;

use super::sink::Sink;
use super::{adapters, ParseState, RunRequest, StreamEvent};

const STDERR_TAIL_LINES: usize = 20;

#[derive(Default)]
pub struct Runs(Mutex<HashMap<String, oneshot::Sender<()>>>);

impl Runs {
    fn register(&self, run_id: String, cancel: oneshot::Sender<()>) {
        self.0.lock().unwrap().insert(run_id, cancel);
    }

    fn finish(&self, run_id: &str) {
        self.0.lock().unwrap().remove(run_id);
    }

    pub fn cancel(&self, run_id: &str) -> bool {
        match self.0.lock().unwrap().remove(run_id) {
            Some(cancel) => cancel.send(()).is_ok(),
            None => false,
        }
    }
}

pub async fn run(app: tauri::AppHandle, req: RunRequest, sink: Sink) -> Result<(), String> {
    use tauri::Manager;

    let adapter = adapters::for_provider(&req.provider_id)
        .ok_or_else(|| format!("unknown provider `{}`", req.provider_id))?;
    let invocation = adapter.invocation(&req);

    let mut command = Command::new(&invocation.program);
    command
        .args(&invocation.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    // A pinned folder can be renamed or unmounted between turns, and spawning
    // into one that is gone reads as the binary being missing.
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
        format!(
            "could not start `{}`: {err}. Is it installed and on PATH?",
            invocation.program
        )
    })?;

    // Written from a task so a prompt larger than the pipe buffer cannot
    // deadlock against the child filling stdout.
    if let (Some(mut stdin), Some(text)) = (child.stdin.take(), invocation.stdin) {
        tokio::spawn(async move {
            let _ = stdin.write_all(text.as_bytes()).await;
            let _ = stdin.shutdown().await;
        });
    }

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

    let (cancel_tx, mut cancel_rx) = oneshot::channel();
    app.state::<Runs>().register(req.run_id.clone(), cancel_tx);

    sink.send(StreamEvent::Start {
        run_id: req.run_id.clone(),
        conversation_id: sink.conversation_id(),
        provider_id: req.provider_id.clone(),
        prompt: req.prompt.clone(),
    })?;

    let stdout = child.stdout.take().expect("stdout piped");
    let mut lines = BufReader::new(stdout).lines();
    let mut state = ParseState::default();
    let mut cancelled = false;

    loop {
        tokio::select! {
            line = lines.next_line() => match line {
                Ok(Some(line)) => {
                    for event in adapter.parse_line(&line, &mut state, &req) {
                        sink.send(event)?;
                    }
                }
                Ok(None) => break,
                Err(err) => {
                    sink.send(StreamEvent::Error {
                        run_id: req.run_id.clone(),
                        message: format!("could not read output: {err}"),
                        stderr_tail: tail_text(&stderr_tail),
                    })?;
                    break;
                }
            },
            _ = &mut cancel_rx => {
                cancelled = true;
                let _ = child.start_kill();
                break;
            }
        }
    }

    let status = child.wait().await;
    app.state::<Runs>().finish(&req.run_id);

    if state.ended {
        return Ok(());
    }

    // Stopping keeps whatever streamed in rather than discarding the turn.
    if cancelled {
        return sink.send(StreamEvent::End {
            run_id: req.run_id.clone(),
            text: state.text.clone(),
            session_id: state.session_id.clone(),
            usage: None,
        });
    }

    let message = match status {
        Ok(status) if status.success() => {
            "the provider exited without returning a result".to_owned()
        }
        Ok(status) => format!("`{}` exited with {status}", invocation.program),
        Err(err) => format!("`{}` could not be waited on: {err}", invocation.program),
    };

    sink.send(StreamEvent::Error {
        run_id: req.run_id.clone(),
        message,
        stderr_tail: tail_text(&stderr_tail),
    })
}

fn tail_text(tail: &Arc<Mutex<VecDeque<String>>>) -> String {
    tail.lock()
        .unwrap()
        .iter()
        .cloned()
        .collect::<Vec<_>>()
        .join("\n")
}
