//! A second, cheap run whose whole job is to name the conversation.
//!
//! The first-line title lands the moment a question is asked, so the sidebar is
//! never blank; this replaces it a few seconds later with something written
//! rather than transcribed. It is deliberately not `cli::run`: that needs an
//! `ipc::Channel` to stream into, and its `Sink` would persist the title as an
//! assistant message in the thread it is naming.

use std::process::Stdio;
use std::time::Duration;

use tauri::Emitter;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use super::{adapters, Invocation};
use crate::db;

/// Long enough for a cold start of the CLI, short enough that a hung binary is
/// not left holding a process for the rest of the session.
const TIMEOUT: Duration = Duration::from_secs(60);

/// How much of each side the namer is shown. A title needs the subject, not
/// the whole answer, and the point of this run is that it is cheap.
const EXCERPT_CHARS: usize = 600;

pub async fn name_conversation(
    app: tauri::AppHandle,
    provider_id: String,
    conversation_id: String,
) {
    match write_title(&app, &provider_id, &conversation_id).await {
        Ok(Some(title)) => log::info!("named conversation {conversation_id}: {title}"),
        Ok(None) => {}
        // Never surfaced: the conversation already has a serviceable title, and
        // a failure here is not something the user asked for or can act on.
        Err(err) => log::warn!("could not name conversation {conversation_id}: {err}"),
    }
}

async fn write_title(
    app: &tauri::AppHandle,
    provider_id: &str,
    conversation_id: &str,
) -> Result<Option<String>, String> {
    let Some(adapter) = adapters::for_provider(provider_id) else {
        return Ok(None);
    };

    let id = conversation_id.to_owned();
    let Some(thread) = db::query(app, move |db| db.thread(&id)).await? else {
        return Ok(None);
    };

    // Only the opening exchange is named. A second answer means the user has
    // been reading this thread under a title, and renaming it under them is
    // worse than the first line they typed.
    let mut answers = thread
        .messages
        .iter()
        .filter(|m| m.role == "assistant" && m.error.is_none());
    let (Some(answer), None) = (answers.next(), answers.next()) else {
        return Ok(None);
    };
    let Some(question) = thread.messages.iter().find(|m| m.role == "user") else {
        return Ok(None);
    };

    let exchange = format!(
        "Question:\n{}\n\nAnswer:\n{}",
        excerpt(&question.text),
        excerpt(&answer.text)
    );

    let Some(title) = clean(&run(adapter.title_invocation(&exchange)).await?) else {
        return Ok(None);
    };

    let id = conversation_id.to_owned();
    let named = title.clone();
    db::query(app, move |db| db.rename_conversation(&id, &named)).await?;
    let _ = app.emit(db::CHANGED_EVENT, ());
    Ok(Some(title))
}

async fn run(invocation: Invocation) -> Result<String, String> {
    let mut command = Command::new(&invocation.program);
    command
        .args(&invocation.args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = command.spawn().map_err(|err| err.to_string())?;
    if let (Some(mut stdin), Some(text)) = (child.stdin.take(), invocation.stdin) {
        let _ = stdin.write_all(text.as_bytes()).await;
        drop(stdin);
    }

    let output = tokio::time::timeout(TIMEOUT, child.wait_with_output())
        .await
        .map_err(|_| "timed out".to_owned())?
        .map_err(|err| err.to_string())?;

    if !output.status.success() {
        return Err(format!("exited with {}", output.status));
    }
    String::from_utf8(output.stdout).map_err(|err| err.to_string())
}

fn excerpt(text: &str) -> String {
    text.chars().take(EXCERPT_CHARS).collect()
}

/// A model asked for four words can still answer in a sentence. The first
/// non-empty line is the title; `rename_conversation` bounds the rest.
fn clean(output: &str) -> Option<String> {
    let line = output
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())?;
    let line = line.trim_matches(['"', '\'', '`', '*']).trim();
    (!line.is_empty()).then(|| line.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn takes_the_first_line_and_drops_what_wraps_it() {
        assert_eq!(clean("Spectral classes"), Some("Spectral classes".into()));
        assert_eq!(
            clean("\n\n  \"Orbit mechanics\"  \n"),
            Some("Orbit mechanics".into())
        );
        assert_eq!(
            clean("**Rebasing**\nand some chatter"),
            Some("Rebasing".into())
        );
        assert_eq!(clean("   \n \n"), None);
        assert_eq!(clean(""), None);
    }
}
