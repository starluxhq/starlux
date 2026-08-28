use std::path::{Path, PathBuf};

use tauri::ipc::Channel;
use tauri::{AppHandle, Emitter, Manager, Window};

use crate::db::{self, Conversation, Message, Thread};
use crate::engine::cli::Runs;
use crate::engine::providers::{self, Provider};
use crate::engine::sink::Sink;
use crate::engine::title;
use crate::engine::tools::{self, Tools};
use crate::engine::{self, RateLimit, RunRequest, StreamEvent};
use crate::state::AppState;
use crate::windows;

#[tauri::command]
pub fn hide_quickbar(app: AppHandle) -> Result<(), String> {
    windows::hide_quickbar(&app).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn toggle_quickbar(app: AppHandle) -> Result<(), String> {
    windows::toggle_quickbar(&app).map_err(|e| e.to_string())
}

/// The bar measures its own content and asks for that height; `windows` bounds it.
#[tauri::command]
pub fn set_quickbar_height(app: AppHandle, height: f64) -> Result<(), String> {
    windows::set_quickbar_height(&app, height).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_workspace(app: AppHandle) -> Result<(), String> {
    windows::open_workspace(&app).map_err(|e| e.to_string())
}

/// Keeps the Quick Bar from hiding while a native dialog or drag has focus.
#[tauri::command]
pub fn set_blur_hide_suppressed(state: tauri::State<'_, AppState>, suppressed: bool) {
    state.set_blur_hide_suppressed(suppressed);
}

/// Probing asks each installed binary whether anyone is signed in to it, which
/// is a subprocess and so cannot run on the thread answering IPC.
#[tauri::command]
pub async fn list_providers() -> Result<Vec<Provider>, String> {
    tauri::async_runtime::spawn_blocking(providers::detect)
        .await
        .map_err(|err| err.to_string())
}

/// What the providers last said about the user's subscription windows. Read at
/// startup so the bar has something to show before the first run of a launch
/// refreshes it.
#[tauri::command]
pub async fn rate_limits(app: AppHandle) -> Result<Vec<RateLimit>, String> {
    db::query(&app, |db| db.rate_limits()).await
}

#[derive(Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Selection {
    provider_id: String,
    model: String,
    /// `None` where nothing was chosen, and where the model offers no choice.
    effort: Option<String>,
}

/// The model a run asks for, remembered across restarts. Kept apart from the
/// model a conversation reports having used: one is a choice, the other is a
/// record, and they are not written in the same vocabulary — `opus` is asked
/// for, `claude-opus-5` comes back.
#[tauri::command]
pub async fn selected_model(app: AppHandle) -> Result<Option<Selection>, String> {
    db::query(&app, |db| {
        Ok(
            match (
                db.setting(db::SELECTED_PROVIDER)?,
                db.setting(db::SELECTED_MODEL)?,
            ) {
                (Some(provider_id), Some(model)) => Some(Selection {
                    provider_id,
                    model,
                    effort: db.setting(db::SELECTED_EFFORT)?,
                }),
                _ => None,
            },
        )
    })
    .await
}

#[tauri::command]
pub async fn set_selected_model(
    app: AppHandle,
    window: Window,
    provider_id: String,
    model: String,
    effort: Option<String>,
) -> Result<(), String> {
    let chosen = Selection {
        provider_id: provider_id.clone(),
        model: model.clone(),
        effort: effort.clone(),
    };
    db::query(&app, move |db| {
        db.set_setting(db::SELECTED_PROVIDER, Some(&provider_id))?;
        db.set_setting(db::SELECTED_MODEL, Some(&model))?;
        db.set_setting(db::SELECTED_EFFORT, effort.as_deref())?;
        db.remember_model(&provider_id, &model)
    })
    .await?;

    // To the other window only: the one that made the choice already has it,
    // and telling it back would have the two windows answering each other.
    let _ = app.emit_to(
        windows::peer_of(window.label()),
        db::SELECTION_EVENT,
        chosen,
    );
    Ok(())
}

/// What each provider was last asked for. Picking a provider still has to pick
/// a model, and the one that sorts first is rarely the one you were using.
#[tauri::command]
pub async fn remembered_models(app: AppHandle) -> Result<Vec<Selection>, String> {
    db::query(&app, |db| {
        Ok(db
            .remembered_models()?
            .into_iter()
            // No level comes back with them: the levels belong to the model,
            // and returning to a provider returns to a model whose ladder may
            // not have the rung you left on.
            .map(|(provider_id, model)| Selection {
                provider_id,
                model,
                effort: None,
            })
            .collect())
    })
    .await
}

/// Remembered across restarts so the Workspace comes back the way it was left.
#[tauri::command]
pub async fn sidebar_collapsed(app: AppHandle) -> Result<bool, String> {
    db::query(&app, |db| {
        Ok(db.setting(db::SIDEBAR_COLLAPSED)?.as_deref() == Some("1"))
    })
    .await
}

#[tauri::command]
pub async fn set_sidebar_collapsed(app: AppHandle, collapsed: bool) -> Result<(), String> {
    db::query(&app, move |db| {
        db.set_setting(db::SIDEBAR_COLLAPSED, collapsed.then_some("1"))
    })
    .await
}

#[tauri::command]
pub fn active_conversation(state: tauri::State<'_, AppState>) -> Option<String> {
    state.active_conversation()
}

#[tauri::command]
pub async fn list_conversations(app: AppHandle) -> Result<Vec<Conversation>, String> {
    db::query(&app, |db| db.list_conversations()).await
}

#[tauri::command]
pub async fn load_conversation(app: AppHandle, id: String) -> Result<Option<Thread>, String> {
    db::query(&app, move |db| db.thread(&id)).await
}

#[tauri::command]
pub async fn rename_conversation(app: AppHandle, id: String, title: String) -> Result<(), String> {
    db::query(&app, move |db| db.rename_conversation(&id, &title)).await?;
    let _ = app.emit(db::CHANGED_EVENT, ());
    Ok(())
}

#[tauri::command]
pub async fn delete_conversation(app: AppHandle, id: String) -> Result<(), String> {
    let state = app.state::<AppState>();
    if state.active_conversation().as_deref() == Some(id.as_str()) {
        state.clear_active_conversation();
    }
    db::query(&app, move |db| {
        if db.setting(db::ACTIVE_CONVERSATION)?.as_deref() == Some(id.as_str()) {
            db.set_setting(db::ACTIVE_CONVERSATION, None)?;
        }
        db.delete_conversation(&id)
    })
    .await?;
    let _ = app.emit(db::CHANGED_EVENT, ());
    Ok(())
}

/// Drops every message after this one, so a retried answer or an edited
/// question is a rewrite of the thread rather than an addition to it. The
/// window cannot do this itself: `load_conversation` reads the thread back out
/// of SQLite, so a client-side splice is undone the next time it is opened.
#[tauri::command]
pub async fn truncate_after(
    app: AppHandle,
    conversation_id: String,
    message_id: String,
) -> Result<(), String> {
    db::query(&app, move |db| {
        db.truncate_after(&conversation_id, &message_id)
    })
    .await?;
    let _ = app.emit(db::CHANGED_EVENT, ());
    Ok(())
}

/// Sorts a conversation above the rest, and keeps it there across restarts.
#[tauri::command]
pub async fn set_pinned(app: AppHandle, id: String, pinned: bool) -> Result<(), String> {
    db::query(&app, move |db| db.set_pinned(&id, pinned)).await?;
    let _ = app.emit(db::CHANGED_EVENT, ());
    Ok(())
}

/// Pins a conversation's runs to a folder, or with `None` returns it to
/// chat-only. Writing it here rather than passing it with each run is what
/// keeps a grant revoked in one window from being spent in the other.
#[tauri::command]
pub async fn set_agent_dir(app: AppHandle, id: String, dir: Option<String>) -> Result<(), String> {
    if let Some(dir) = &dir {
        if !Path::new(dir).is_dir() {
            return Err(format!("`{dir}` is not a folder"));
        }
    }
    db::query(&app, move |db| db.set_agent_dir(&id, dir.as_deref())).await?;
    let _ = app.emit(db::CHANGED_EVENT, ());
    Ok(())
}

/// What every run may reach beyond the model itself. One answer for the app
/// rather than one per conversation: a question asked from the bar reaches
/// exactly what one asked from the Workspace does.
#[tauri::command]
pub async fn tools(app: AppHandle) -> Result<Tools, String> {
    db::query(&app, |db| db.tools()).await
}

/// Grants a single tool, or takes it back. The id is checked against the ones
/// Starlux defines: a window naming something else is not a grant, and a row
/// nobody can read back would be a grant that never applies.
#[tauri::command]
pub async fn set_tool(
    app: AppHandle,
    window: Window,
    id: String,
    on: bool,
) -> Result<Tools, String> {
    if !tools::ALL.contains(&id.as_str()) {
        return Err(format!("`{id}` is not a tool Starlux knows"));
    }
    let granted = db::query(&app, move |db| {
        db.set_tool(&id, on)?;
        db.tools()
    })
    .await?;

    // To the other window only, for the reason `set_selected_model` gives.
    let _ = app.emit_to(windows::peer_of(window.label()), db::TOOLS_EVENT, granted);
    Ok(granted)
}

#[tauri::command]
pub async fn run_prompt(
    app: AppHandle,
    window: Window,
    mut request: RunRequest,
    on_event: Channel<StreamEvent>,
) -> Result<(), String> {
    let conversation_id = request.conversation_id.clone();
    let provider_id = request.provider_id.clone();
    let prompt = request.prompt.clone();
    let asked = prompt.clone();
    let agent_dir = request
        .agent_dir
        .as_ref()
        .map(|dir| dir.to_string_lossy().into_owned());
    let attached = request.attachments.clone();
    let question = Message {
        id: format!("{}:u", request.run_id),
        role: "user".to_owned(),
        text: prompt.clone(),
        model: None,
        usage: None,
        error: None,
        // Described rather than read: the row is written before the run starts,
        // and it is the run that decides whether a file it cannot open is fatal.
        attachments: attached.iter().map(|path| engine::describe(path)).collect(),
    };

    // Written before the process starts, so a run that dies still leaves the
    // question in history rather than a conversation that never happened.
    //
    // Both halves of the grant come back out of the database because they, not
    // the request, decide what this run may reach: the folder is the one this
    // conversation was given, and the tools are the ones the app is set to.
    let id = conversation_id.clone();
    let named_by = provider_id.clone();
    let (started, folder, granted) = db::query(&app, move |db| {
        let started = db.ensure_conversation(&id, &prompt, &provider_id, agent_dir.as_deref())?;
        db.set_setting(db::ACTIVE_CONVERSATION, Some(&id))?;
        db.add_message(&id, &question)?;
        Ok((started, db.agent_dir(&id)?, db.tools()?))
    })
    .await?;
    request.agent_dir = folder.map(PathBuf::from);
    request.tools = granted;
    let _ = app.emit(db::CHANGED_EVENT, ());

    app.state::<AppState>()
        .set_active_conversation(conversation_id.clone());

    let sink = Sink::new(
        app.clone(),
        window.label().to_owned(),
        on_event,
        conversation_id.clone(),
        request.model.clone(),
    );

    // Sent alongside the question rather than after the answer, so the written
    // title lands while the first answer is still streaming. Only the run that
    // opened the conversation names it: a follow-up is not a reason to retitle
    // a thread the user has been reading.
    if started {
        tauri::async_runtime::spawn(title::name_conversation(
            app.clone(),
            named_by,
            conversation_id,
            asked,
        ));
    }

    engine::run(app, request, sink).await
}

#[tauri::command]
pub fn cancel_run(state: tauri::State<'_, Runs>, run_id: String) -> bool {
    state.cancel(&run_id)
}

/// Hands an interactive answer to the `artifact:` scheme and returns its id, so
/// the webview can frame it as a document with its own policy.
#[tauri::command]
pub fn store_artifact(app: AppHandle, html: String) -> String {
    app.state::<crate::artifacts::Artifacts>().put(html)
}
