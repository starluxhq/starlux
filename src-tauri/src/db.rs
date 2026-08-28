use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::engine::tools::{self, Tools};
use crate::engine::{self, Attachment, Context, RateLimit, Usage};

const SCHEMA_V1: &str = "
CREATE TABLE conversations (
  id                  TEXT PRIMARY KEY,
  title               TEXT NOT NULL,
  provider_id         TEXT NOT NULL,
  provider_session_id TEXT,
  model               TEXT,
  agent_dir           TEXT,
  pinned              INTEGER NOT NULL DEFAULT 0,
  created_at          INTEGER NOT NULL,
  updated_at          INTEGER NOT NULL
);

CREATE TABLE messages (
  id              TEXT PRIMARY KEY,
  conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
  role            TEXT NOT NULL,
  content         TEXT NOT NULL,
  model           TEXT,
  tokens_in       INTEGER,
  tokens_out      INTEGER,
  cost_usd        REAL,
  error           TEXT,
  created_at      INTEGER NOT NULL
);

CREATE INDEX messages_by_conversation ON messages(conversation_id, created_at);

CREATE TABLE attachments (
  id         TEXT PRIMARY KEY,
  message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
  path       TEXT NOT NULL,
  mime       TEXT,
  bytes      INTEGER
);

CREATE TABLE settings (
  key   TEXT PRIMARY KEY,
  value TEXT NOT NULL
);
";

const TITLE_MAX_CHARS: usize = 64;

/// Emitted after every write so open sidebars re-read the list.
pub const CHANGED_EVENT: &str = "starlux://conversations";
/// Emitted to the *other* window when this one changes what the next run asks
/// for. The choice outlives any one conversation, so both windows hold it, and
/// a window that never heard about the change shows a stale one.
pub const SELECTION_EVENT: &str = "starlux://selection";
/// Emitted to the *other* window when this one changes what runs may reach.
/// One answer for the whole app, so both windows show it and neither may sit
/// claiming a grant that has been given back.
pub const TOOLS_EVENT: &str = "starlux://tools";

/// The thread the Workspace comes back to after a restart.
pub const ACTIVE_CONVERSATION: &str = "active_conversation";
/// What the next run will ask for, which outlives any one conversation.
pub const SELECTED_PROVIDER: &str = "selected_provider";
pub const SELECTED_MODEL: &str = "selected_model";
/// How hard the chosen model is asked to think. Absent means the provider's
/// own default, which is not the same as any level it offers.
pub const SELECTED_EFFORT: &str = "selected_effort";
/// Whether the Workspace's conversation list is slid away.
pub const SIDEBAR_COLLAPSED: &str = "sidebar_collapsed";
/// One row per provider holding the model last asked of it, so switching
/// provider and back returns to what you were using rather than to whatever
/// sorts first.
const MODEL_PREFIX: &str = "model:";
/// One row per provider holding only its latest window. A snapshot of something
/// that moves, not a history: `messages` is where anything worth keeping goes.
const RATE_LIMIT_PREFIX: &str = "rate_limit:";
/// One row per tool the user has switched on. Not on the conversation: what a
/// run may reach is one answer for the whole app, so a question asked from the
/// bar reaches exactly what one asked from the Workspace does.
const TOOL_PREFIX: &str = "tool:";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    pub id: String,
    pub title: String,
    pub provider_id: String,
    pub session_id: Option<String>,
    pub model: Option<String>,
    pub agent_dir: Option<String>,
    pub updated_at: i64,
    pub pinned: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: String,
    pub role: String,
    pub text: String,
    pub model: Option<String>,
    pub usage: Option<Usage>,
    pub error: Option<String>,
    /// What was attached to the question. Rewritten wholesale with the message,
    /// so asking again under the same id re-sends the same files rather than
    /// accumulating a second copy of them.
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Thread {
    pub conversation: Conversation,
    pub messages: Vec<Message>,
}

pub struct Db(Mutex<Connection>);

impl Db {
    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        Self::init(Connection::open(path)?)
    }

    fn init(mut conn: Connection) -> rusqlite::Result<Self> {
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        migrate(&mut conn)?;
        Ok(Self(Mutex::new(conn)))
    }

    pub fn ensure_conversation(
        &self,
        id: &str,
        title_source: &str,
        provider_id: &str,
        agent_dir: Option<&str>,
    ) -> rusqlite::Result<bool> {
        let conn = self.0.lock().unwrap();
        let now = now_ms();
        let changed = conn.execute(
            "INSERT INTO conversations
               (id, title, provider_id, agent_dir, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?5)
             ON CONFLICT(id) DO NOTHING",
            params![id, title_from(title_source), provider_id, agent_dir, now],
        )?;
        Ok(changed == 1)
    }

    pub fn add_message(&self, conversation_id: &str, message: &Message) -> rusqlite::Result<()> {
        let conn = self.0.lock().unwrap();
        let now = now_ms();
        conn.execute(
            "INSERT INTO messages
               (id, conversation_id, role, content, model, tokens_in, tokens_out, cost_usd,
                context_used, context_window, error, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(id) DO UPDATE SET
               content = excluded.content, model = excluded.model,
               tokens_in = excluded.tokens_in, tokens_out = excluded.tokens_out,
               cost_usd = excluded.cost_usd, context_used = excluded.context_used,
               context_window = excluded.context_window, error = excluded.error",
            params![
                message.id,
                conversation_id,
                message.role,
                message.text,
                message.model,
                message.usage.as_ref().map(|u| u.input_tokens as i64),
                message.usage.as_ref().map(|u| u.output_tokens as i64),
                message.usage.as_ref().and_then(|u| u.cost_usd),
                message
                    .usage
                    .as_ref()
                    .and_then(|u| u.context)
                    .map(|c| c.used as i64),
                message
                    .usage
                    .as_ref()
                    .and_then(|u| u.context)
                    .map(|c| c.window as i64),
                message.error,
                now,
            ],
        )?;
        conn.execute(
            "DELETE FROM attachments WHERE message_id = ?1",
            params![message.id],
        )?;
        for (at, file) in message.attachments.iter().enumerate() {
            conn.execute(
                "INSERT INTO attachments (id, message_id, path, mime, bytes)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    format!("{}:{at}", message.id),
                    message.id,
                    file.path,
                    file.mime,
                    file.bytes
                ],
            )?;
        }
        conn.execute(
            "UPDATE conversations SET updated_at = ?2 WHERE id = ?1",
            params![conversation_id, now],
        )?;
        Ok(())
    }

    pub fn set_session(
        &self,
        conversation_id: &str,
        session_id: &str,
        model: Option<&str>,
    ) -> rusqlite::Result<()> {
        let conn = self.0.lock().unwrap();
        conn.execute(
            "UPDATE conversations
                SET provider_session_id = ?2, model = COALESCE(?3, model)
              WHERE id = ?1",
            params![conversation_id, session_id, model],
        )?;
        Ok(())
    }

    /// The conversation's provider is the one that last answered in it, not the
    /// one it opened on. Switching provider mid-thread wrote the new provider's
    /// session and model into a row that still named the old one, and every
    /// thread load — which each expand does — read that row back: the picker
    /// silently returned to the provider left behind, holding a session id the
    /// other CLI had issued.
    ///
    /// The session and model go with it. They belong to the provider that
    /// issued them, and a run interrupted between here and its first answer
    /// would otherwise leave the pair mismatched on disk.
    pub fn set_provider(&self, id: &str, provider_id: &str) -> rusqlite::Result<()> {
        let conn = self.0.lock().unwrap();
        conn.execute(
            "UPDATE conversations
                SET provider_id = ?2, provider_session_id = NULL, model = NULL
              WHERE id = ?1 AND provider_id <> ?2",
            params![id, provider_id],
        )?;
        Ok(())
    }

    /// `None` returns the conversation to chat-only. The column is the only
    /// record of the grant, so a run reads it back rather than trusting the
    /// window that asked.
    pub fn set_agent_dir(&self, id: &str, dir: Option<&str>) -> rusqlite::Result<()> {
        let conn = self.0.lock().unwrap();
        conn.execute(
            "UPDATE conversations SET agent_dir = ?2 WHERE id = ?1",
            params![id, dir],
        )?;
        Ok(())
    }

    /// A pinned conversation sorts above the rest; `list_conversations` has
    /// always ordered by it, and this is what finally writes it.
    pub fn set_pinned(&self, id: &str, pinned: bool) -> rusqlite::Result<()> {
        let conn = self.0.lock().unwrap();
        conn.execute(
            "UPDATE conversations SET pinned = ?2 WHERE id = ?1",
            params![id, pinned],
        )?;
        Ok(())
    }

    /// What a run may reach, read back rather than taken from the window that
    /// asked. A conversation nobody granted anything reads as chat-only.
    pub fn agent_dir(&self, id: &str) -> rusqlite::Result<Option<String>> {
        let conn = self.0.lock().unwrap();
        Ok(conn
            .query_row(
                "SELECT agent_dir FROM conversations WHERE id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()?
            .flatten())
    }

    pub fn list_conversations(&self) -> rusqlite::Result<Vec<Conversation>> {
        let conn = self.0.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, provider_id, provider_session_id, model, agent_dir,
                    updated_at, pinned
               FROM conversations
              ORDER BY pinned DESC, updated_at DESC",
        )?;
        let rows = stmt.query_map([], read_conversation)?;
        rows.collect()
    }

    pub fn thread(&self, conversation_id: &str) -> rusqlite::Result<Option<Thread>> {
        let conn = self.0.lock().unwrap();
        let conversation = conn
            .query_row(
                "SELECT id, title, provider_id, provider_session_id, model, agent_dir,
                    updated_at, pinned
                   FROM conversations WHERE id = ?1",
                params![conversation_id],
                read_conversation,
            )
            .optional()?;

        let Some(conversation) = conversation else {
            return Ok(None);
        };

        let mut stmt = conn.prepare(
            "SELECT id, role, content, model, tokens_in, tokens_out, cost_usd,
                    context_used, context_window, error
               FROM messages WHERE conversation_id = ?1
              ORDER BY created_at, rowid",
        )?;
        let mut messages = stmt
            .query_map(params![conversation_id], read_message)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        let mut stmt = conn.prepare(
            "SELECT a.message_id, a.path, a.mime, a.bytes
               FROM attachments a JOIN messages m ON m.id = a.message_id
              WHERE m.conversation_id = ?1
              ORDER BY a.rowid",
        )?;
        let files = stmt
            .query_map(params![conversation_id], |row| {
                Ok((row.get::<_, String>(0)?, read_attachment(row)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for (message_id, file) in files {
            if let Some(message) = messages.iter_mut().find(|m| m.id == message_id) {
                message.attachments.push(file);
            }
        }

        Ok(Some(Thread {
            conversation,
            messages,
        }))
    }

    /// Drops everything that follows one message, which is what makes editing a
    /// question or retrying an answer a rewrite rather than an append. Ordered
    /// the way `thread` reads, so what is deleted is what was shown below it.
    pub fn truncate_after(&self, conversation_id: &str, message_id: &str) -> rusqlite::Result<()> {
        let conn = self.0.lock().unwrap();
        conn.execute(
            "DELETE FROM messages
              WHERE conversation_id = ?1
                AND (created_at, rowid) > (SELECT created_at, rowid FROM messages WHERE id = ?2)",
            params![conversation_id, message_id],
        )?;
        Ok(())
    }

    pub fn set_setting(&self, key: &str, value: Option<&str>) -> rusqlite::Result<()> {
        let conn = self.0.lock().unwrap();
        match value {
            Some(value) => conn.execute(
                "INSERT INTO settings (key, value) VALUES (?1, ?2)
                 ON CONFLICT(key) DO UPDATE SET value = excluded.value",
                params![key, value],
            ),
            None => conn.execute("DELETE FROM settings WHERE key = ?1", params![key]),
        }?;
        Ok(())
    }

    pub fn setting(&self, key: &str) -> rusqlite::Result<Option<String>> {
        let conn = self.0.lock().unwrap();
        conn.query_row(
            "SELECT value FROM settings WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .optional()
    }

    /// Every tool, granted or not, so the answer is the whole grant rather than
    /// whichever rows happen to exist. An id nobody defined is not stored.
    pub fn tools(&self) -> rusqlite::Result<Tools> {
        let mut granted = Tools::default();
        for id in tools::ALL {
            granted.set(
                id,
                self.setting(&format!("{TOOL_PREFIX}{id}"))?.as_deref() == Some("1"),
            );
        }
        Ok(granted)
    }

    pub fn set_tool(&self, id: &str, on: bool) -> rusqlite::Result<()> {
        self.set_setting(&format!("{TOOL_PREFIX}{id}"), on.then_some("1"))
    }

    pub fn remember_model(&self, provider_id: &str, model: &str) -> rusqlite::Result<()> {
        self.set_setting(&format!("{MODEL_PREFIX}{provider_id}"), Some(model))
    }

    pub fn remembered_models(&self) -> rusqlite::Result<Vec<(String, String)>> {
        let conn = self.0.lock().unwrap();
        let mut statement = conn.prepare("SELECT key, value FROM settings WHERE key LIKE ?1")?;
        let rows = statement.query_map(params![format!("{MODEL_PREFIX}%")], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?;
        rows.map(|row| row.map(|(key, model)| (key[MODEL_PREFIX.len()..].to_owned(), model)))
            .collect()
    }

    pub fn set_rate_limit(&self, limit: &RateLimit) -> rusqlite::Result<()> {
        let Ok(value) = serde_json::to_string(limit) else {
            return Ok(());
        };
        self.set_setting(
            &format!("{RATE_LIMIT_PREFIX}{}", limit.provider_id),
            Some(&value),
        )
    }

    pub fn rate_limits(&self) -> rusqlite::Result<Vec<RateLimit>> {
        let conn = self.0.lock().unwrap();
        let mut statement = conn.prepare("SELECT value FROM settings WHERE key LIKE ?1")?;
        let rows = statement.query_map(params![format!("{RATE_LIMIT_PREFIX}%")], |row| {
            row.get::<_, String>(0)
        })?;
        Ok(rows
            .filter_map(Result::ok)
            // A row an older build wrote in a shape this one cannot read is
            // dropped, not fatal: it is a cache of something a run refreshes.
            .filter_map(|value| serde_json::from_str(&value).ok())
            .collect())
    }

    pub fn rename_conversation(&self, id: &str, title: &str) -> rusqlite::Result<()> {
        let conn = self.0.lock().unwrap();
        conn.execute(
            "UPDATE conversations SET title = ?2 WHERE id = ?1",
            params![id, title_from(title)],
        )?;
        Ok(())
    }

    pub fn delete_conversation(&self, id: &str) -> rusqlite::Result<()> {
        let conn = self.0.lock().unwrap();
        conn.execute("DELETE FROM conversations WHERE id = ?1", params![id])?;
        Ok(())
    }
}

/// One transaction per step, so a migration that fails leaves the database on
/// its previous version instead of half-built and unopenable.
/// Answers written before this ran keep NULL, which reads back as "we do not
/// know how full that conversation was" rather than as an empty one.
const SCHEMA_V2: &str = "
ALTER TABLE messages ADD COLUMN context_used INTEGER;
ALTER TABLE messages ADD COLUMN context_window INTEGER;
";

/// Conversations that existed before the grant was split keep the answer the
/// old binary would have given: chat-only, or a folder, and no web either way.
const SCHEMA_V3: &str = "
ALTER TABLE conversations ADD COLUMN web INTEGER NOT NULL DEFAULT 0;
";

/// The web grant stopped being a property of a conversation and became one of
/// the app, so the column goes rather than sitting there outranked. Nothing is
/// carried over: every tool starts off, which is the safe direction to be wrong
/// in and the same place a fresh install starts.
const SCHEMA_V4: &str = "
ALTER TABLE conversations DROP COLUMN web;
";

fn migrate(conn: &mut Connection) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    let version: i64 = tx.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version < 1 {
        tx.execute_batch(SCHEMA_V1)?;
        tx.pragma_update(None, "user_version", 1)?;
    }
    if version < 2 {
        tx.execute_batch(SCHEMA_V2)?;
        tx.pragma_update(None, "user_version", 2)?;
    }
    if version < 3 {
        tx.execute_batch(SCHEMA_V3)?;
        tx.pragma_update(None, "user_version", 3)?;
    }
    if version < 4 {
        tx.execute_batch(SCHEMA_V4)?;
        tx.pragma_update(None, "user_version", 4)?;
    }
    tx.commit()
}

fn read_conversation(row: &rusqlite::Row<'_>) -> rusqlite::Result<Conversation> {
    Ok(Conversation {
        id: row.get(0)?,
        title: row.get(1)?,
        provider_id: row.get(2)?,
        session_id: row.get(3)?,
        model: row.get(4)?,
        agent_dir: row.get(5)?,
        updated_at: row.get(6)?,
        pinned: row.get(7)?,
    })
}

/// The name is derived rather than stored: it is the last segment of the path,
/// and a second copy of it could only ever disagree with the first.
fn read_attachment(row: &rusqlite::Row<'_>) -> rusqlite::Result<Attachment> {
    let path: String = row.get(1)?;
    Ok(Attachment {
        name: engine::file_name(Path::new(&path)),
        path,
        mime: row.get(2)?,
        bytes: row.get(3)?,
    })
}

fn read_message(row: &rusqlite::Row<'_>) -> rusqlite::Result<Message> {
    let input_tokens: Option<i64> = row.get(4)?;
    let output_tokens: Option<i64> = row.get(5)?;
    let cost_usd: Option<f64> = row.get(6)?;
    let context_used: Option<i64> = row.get(7)?;
    let context_window: Option<i64> = row.get(8)?;
    Ok(Message {
        id: row.get(0)?,
        role: row.get(1)?,
        text: row.get(2)?,
        model: row.get(3)?,
        usage: input_tokens
            .zip(output_tokens)
            .map(|(input, output)| Usage {
                input_tokens: input as u64,
                output_tokens: output as u64,
                cost_usd,
                context: context_used
                    .zip(context_window)
                    .map(|(used, window)| Context {
                        used: used as u64,
                        window: window as u64,
                    }),
            }),
        error: row.get(9)?,
        attachments: Vec::new(),
    })
}

fn title_from(source: &str) -> String {
    let line = source.trim().lines().next().unwrap_or("").trim();
    if line.is_empty() {
        return "Untitled".to_owned();
    }
    match line.char_indices().nth(TITLE_MAX_CHARS) {
        Some((cut, _)) => format!("{}…", line[..cut].trim_end()),
        None => line.to_owned(),
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as i64)
        .unwrap_or_default()
}

/// Runs a query off the async runtime; `rusqlite` is blocking and would
/// otherwise stall every other task sharing the worker thread.
pub async fn query<T, F>(app: &AppHandle, work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce(&Db) -> rusqlite::Result<T> + Send + 'static,
{
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || work(&app.state::<Db>()))
        .await
        .map_err(|err| err.to_string())?
        .map_err(|err| err.to_string())
}

/// Fire-and-forget variant for writes made while streaming, where losing a row
/// must not abort the run in progress.
pub fn write<F>(app: &AppHandle, work: F)
where
    F: FnOnce(&Db) -> rusqlite::Result<()> + Send + 'static,
{
    let app = app.clone();
    tauri::async_runtime::spawn_blocking(move || match work(&app.state::<Db>()) {
        Ok(()) => {
            let _ = app.emit(CHANGED_EVENT, ());
        }
        Err(err) => log::error!("database write failed: {err}"),
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn db() -> Db {
        Db::init(Connection::open_in_memory().unwrap()).unwrap()
    }

    fn message(id: &str, role: &str, text: &str) -> Message {
        Message {
            id: id.to_owned(),
            role: role.to_owned(),
            text: text.to_owned(),
            model: None,
            usage: None,
            error: None,
            attachments: Vec::new(),
        }
    }

    #[test]
    fn a_failed_migration_leaves_the_database_on_its_old_version() {
        let mut conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE conversations (bogus TEXT)")
            .unwrap();
        assert!(migrate(&mut conn).is_err());
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, 0);
        assert!(conn
            .query_row("SELECT count(*) FROM messages", [], |row| row
                .get::<_, i64>(0))
            .is_err());
    }

    #[test]
    fn stores_and_reads_back_a_thread() {
        let db = db();
        assert!(db
            .ensure_conversation("c1", "How do pulsars work?", "claude-cli", None)
            .unwrap());
        db.add_message("c1", &message("r1:u", "user", "How do pulsars work?"))
            .unwrap();
        db.add_message(
            "c1",
            &Message {
                usage: Some(Usage {
                    input_tokens: 12,
                    output_tokens: 34,
                    cost_usd: Some(0.5),
                    context: Some(Context {
                        used: 26_829,
                        window: 200_000,
                    }),
                }),
                model: Some("opus".to_owned()),
                ..message("r1", "assistant", "They spin.")
            },
        )
        .unwrap();

        let thread = db.thread("c1").unwrap().unwrap();
        assert_eq!(thread.conversation.title, "How do pulsars work?");
        let roles: Vec<_> = thread.messages.iter().map(|m| m.role.as_str()).collect();
        assert_eq!(roles, ["user", "assistant"]);
        assert_eq!(thread.messages[1].model.as_deref(), Some("opus"));
        assert_eq!(thread.messages[1].usage.as_ref().unwrap().output_tokens, 34);
        assert_eq!(
            thread.messages[1].usage.as_ref().unwrap().context,
            Some(Context {
                used: 26_829,
                window: 200_000
            })
        );
    }

    #[test]
    fn an_answer_stored_without_a_context_reading_reports_none() {
        let db = db();
        db.ensure_conversation("c1", "q", "claude-cli", None)
            .unwrap();
        db.add_message(
            "c1",
            &Message {
                usage: Some(Usage {
                    input_tokens: 1,
                    output_tokens: 2,
                    cost_usd: None,
                    context: None,
                }),
                ..message("r1", "assistant", "a")
            },
        )
        .unwrap();

        let thread = db.thread("c1").unwrap().unwrap();
        assert_eq!(thread.messages[0].usage.as_ref().unwrap().context, None);
    }

    // The columns arrived in v2, so a database written before them has to gain
    // them without losing the answers already in it.
    #[test]
    fn a_v1_database_gains_the_context_columns_and_keeps_its_messages() {
        let mut conn = Connection::open_in_memory().unwrap();
        {
            let tx = conn.transaction().unwrap();
            tx.execute_batch(SCHEMA_V1).unwrap();
            tx.pragma_update(None, "user_version", 1).unwrap();
            tx.execute(
                "INSERT INTO conversations (id, title, provider_id, created_at, updated_at)
                 VALUES ('c1', 'old', 'claude-cli', 1, 1)",
                [],
            )
            .unwrap();
            tx.execute(
                "INSERT INTO messages (id, conversation_id, role, content, created_at)
                 VALUES ('m1', 'c1', 'assistant', 'written before v2', 1)",
                [],
            )
            .unwrap();
            tx.commit().unwrap();
        }

        let db = Db::init(conn).unwrap();
        let thread = db.thread("c1").unwrap().unwrap();
        assert_eq!(thread.messages[0].text, "written before v2");
        assert_eq!(thread.messages[0].usage, None);
    }

    fn attached(path: &str) -> Attachment {
        Attachment {
            path: path.to_owned(),
            name: engine::file_name(Path::new(path)),
            mime: Some(engine::mime_of(Path::new(path)).to_owned()),
            bytes: Some(12),
        }
    }

    #[test]
    fn a_question_keeps_the_files_that_were_asked_alongside_it() {
        let db = db();
        db.ensure_conversation("c1", "what is this?", "claude-cli", None)
            .unwrap();
        db.add_message(
            "c1",
            &Message {
                attachments: vec![attached("/home/a/blue.png"), attached("/home/a/notes.md")],
                ..message("r1:u", "user", "what is this?")
            },
        )
        .unwrap();
        db.add_message("c1", &message("r1", "assistant", "blue"))
            .unwrap();

        let thread = db.thread("c1").unwrap().unwrap();
        let files = &thread.messages[0].attachments;
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].name, "blue.png");
        assert_eq!(files[0].mime.as_deref(), Some("image/png"));
        assert_eq!(files[0].bytes, Some(12));
        assert_eq!(files[1].name, "notes.md");
        assert!(thread.messages[1].attachments.is_empty());
    }

    /// An edited question is rewritten under its own id, so its files have to be
    /// replaced rather than joined by a second copy of themselves.
    #[test]
    fn rewriting_a_question_replaces_what_was_attached_to_it() {
        let db = db();
        db.ensure_conversation("c1", "hi", "claude-cli", None)
            .unwrap();
        db.add_message(
            "c1",
            &Message {
                attachments: vec![attached("/a.png")],
                ..message("r1:u", "user", "first")
            },
        )
        .unwrap();
        db.add_message(
            "c1",
            &Message {
                attachments: vec![attached("/b.png"), attached("/c.png")],
                ..message("r1:u", "user", "second")
            },
        )
        .unwrap();

        let files = &db.thread("c1").unwrap().unwrap().messages[0].attachments;
        let names: Vec<_> = files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["b.png", "c.png"]);
    }

    #[test]
    fn truncating_takes_the_attachments_of_the_dropped_turns() {
        let db = db();
        db.ensure_conversation("c1", "hi", "claude-cli", None)
            .unwrap();
        for id in ["r1:u", "r1", "r2:u"] {
            db.add_message(
                "c1",
                &Message {
                    attachments: vec![attached("/a.png")],
                    ..message(id, "user", id)
                },
            )
            .unwrap();
        }

        db.truncate_after("c1", "r1").unwrap();
        let orphans: i64 =
            db.0.lock()
                .unwrap()
                .query_row("SELECT count(*) FROM attachments", [], |row| row.get(0))
                .unwrap();
        assert_eq!(orphans, 2);
    }

    /// A database written before the grant moved keeps its folder and arrives
    /// with nothing granted, which is where a fresh install starts too.
    #[test]
    fn a_database_written_before_the_move_keeps_its_folder_and_no_tools() {
        let mut conn = Connection::open_in_memory().unwrap();
        {
            let tx = conn.transaction().unwrap();
            tx.execute_batch(SCHEMA_V1).unwrap();
            tx.execute_batch(SCHEMA_V2).unwrap();
            tx.pragma_update(None, "user_version", 2).unwrap();
            tx.execute(
                "INSERT INTO conversations (id, title, provider_id, agent_dir, created_at, updated_at)
                 VALUES ('c1', 'old', 'claude-cli', '/work', 1, 1)",
                [],
            )
            .unwrap();
            tx.commit().unwrap();
        }

        let db = Db::init(conn).unwrap();
        assert_eq!(db.agent_dir("c1").unwrap().as_deref(), Some("/work"));
        assert_eq!(db.tools().unwrap(), Tools::default());
    }

    /// The path every existing install takes. The grant itself is not carried
    /// over — off is where a fresh install starts, and the safe direction to be
    /// wrong in — but the conversation it was on must survive intact.
    #[test]
    fn a_conversation_that_had_the_web_grant_survives_losing_it() {
        let mut conn = Connection::open_in_memory().unwrap();
        {
            let tx = conn.transaction().unwrap();
            tx.execute_batch(SCHEMA_V1).unwrap();
            tx.execute_batch(SCHEMA_V2).unwrap();
            tx.execute_batch(SCHEMA_V3).unwrap();
            tx.pragma_update(None, "user_version", 3).unwrap();
            tx.execute(
                "INSERT INTO conversations
                   (id, title, provider_id, agent_dir, web, pinned, created_at, updated_at)
                 VALUES ('c1', 'kept', 'gemini-cli', '/work', 1, 1, 1, 1)",
                [],
            )
            .unwrap();
            tx.commit().unwrap();
        }

        let db = Db::init(conn).unwrap();
        let kept = &db.list_conversations().unwrap()[0];
        assert_eq!(kept.title, "kept");
        assert_eq!(kept.provider_id, "gemini-cli");
        assert_eq!(kept.agent_dir.as_deref(), Some("/work"));
        assert!(kept.pinned);
        assert_eq!(db.tools().unwrap(), Tools::default());
    }

    #[test]
    fn ensure_conversation_is_idempotent() {
        let db = db();
        assert!(db
            .ensure_conversation("c1", "first", "claude-cli", None)
            .unwrap());
        assert!(!db
            .ensure_conversation("c1", "second", "claude-cli", None)
            .unwrap());
        assert_eq!(db.list_conversations().unwrap().len(), 1);
        assert_eq!(
            db.thread("c1").unwrap().unwrap().conversation.title,
            "first"
        );
    }

    #[test]
    fn resuming_a_conversation_recovers_its_provider_session() {
        let db = db();
        db.ensure_conversation("c1", "hi", "claude-cli", None)
            .unwrap();
        db.set_session("c1", "sess-42", Some("sonnet")).unwrap();
        let thread = db.thread("c1").unwrap().unwrap();
        assert_eq!(thread.conversation.session_id.as_deref(), Some("sess-42"));
        assert_eq!(thread.conversation.model.as_deref(), Some("sonnet"));
    }

    /// Without this the row kept the provider the thread opened on while
    /// holding the session id another CLI had issued, and every thread load
    /// handed that pair back to the picker.
    #[test]
    fn switching_provider_moves_the_conversation_and_drops_the_old_session() {
        let db = db();
        db.ensure_conversation("c1", "hi", "claude-cli", None)
            .unwrap();
        db.set_session("c1", "sess-42", Some("opus")).unwrap();

        db.set_provider("c1", "opencode-cli").unwrap();
        let moved = db.thread("c1").unwrap().unwrap().conversation;
        assert_eq!(moved.provider_id, "opencode-cli");
        assert_eq!(moved.session_id, None);
        assert_eq!(moved.model, None);

        db.set_session("c1", "ses_99", Some("opencode/hy3-free"))
            .unwrap();
        db.set_provider("c1", "opencode-cli").unwrap();
        let stayed = db.thread("c1").unwrap().unwrap().conversation;
        assert_eq!(stayed.session_id.as_deref(), Some("ses_99"));
        assert_eq!(stayed.model.as_deref(), Some("opencode/hy3-free"));
    }

    #[test]
    fn a_streamed_answer_can_be_rewritten_in_place() {
        let db = db();
        db.ensure_conversation("c1", "hi", "claude-cli", None)
            .unwrap();
        db.add_message("c1", &message("r1", "assistant", "partial"))
            .unwrap();
        db.add_message("c1", &message("r1", "assistant", "final"))
            .unwrap();
        let thread = db.thread("c1").unwrap().unwrap();
        assert_eq!(thread.messages.len(), 1);
        assert_eq!(thread.messages[0].text, "final");
    }

    #[test]
    fn a_conversation_is_pinned_to_a_folder_and_can_be_released() {
        let db = db();
        db.ensure_conversation("c1", "hi", "claude-cli", Some("/work"))
            .unwrap();
        assert_eq!(db.agent_dir("c1").unwrap().as_deref(), Some("/work"));
        db.set_agent_dir("c1", Some("/elsewhere")).unwrap();
        assert_eq!(db.agent_dir("c1").unwrap().as_deref(), Some("/elsewhere"));
        db.set_agent_dir("c1", None).unwrap();
        assert_eq!(db.agent_dir("c1").unwrap(), None);
        assert_eq!(db.agent_dir("nobody").unwrap(), None);
    }

    /// One answer for the whole app, and one row per tool: granting the fetcher
    /// must not carry search in with it.
    #[test]
    fn each_tool_is_granted_and_given_back_on_its_own() {
        let db = db();
        assert_eq!(db.tools().unwrap(), Tools::default());

        db.set_tool(tools::WEB_FETCH, true).unwrap();
        assert_eq!(
            db.tools().unwrap(),
            Tools {
                web_search: false,
                web_fetch: true
            }
        );

        db.set_tool(tools::WEB_SEARCH, true).unwrap();
        assert!(db.tools().unwrap().web_search);

        db.set_tool(tools::WEB_FETCH, false).unwrap();
        assert_eq!(
            db.tools().unwrap(),
            Tools {
                web_search: true,
                web_fetch: false
            }
        );
    }

    /// The grant left the conversation, and nothing about a conversation should
    /// still be able to answer for it.
    #[test]
    fn a_conversation_no_longer_carries_a_web_grant() {
        let db = db();
        db.ensure_conversation("c1", "hi", "claude-cli", None)
            .unwrap();
        let conn = db.0.lock().unwrap();
        let columns: Vec<String> = conn
            .prepare("SELECT * FROM conversations")
            .unwrap()
            .column_names()
            .iter()
            .map(|name| (*name).to_owned())
            .collect();
        assert!(!columns.contains(&"web".to_owned()), "{columns:?}");
    }

    #[test]
    fn the_chosen_model_outlives_the_conversation_that_used_it() {
        let db = db();
        db.set_setting(SELECTED_PROVIDER, Some("claude-cli"))
            .unwrap();
        db.set_setting(SELECTED_MODEL, Some("opus")).unwrap();

        // A run reports the exact build it used against the conversation. That
        // must not disturb the alias the picker offers and the next run sends.
        db.ensure_conversation("c1", "hello", "claude-cli", None)
            .unwrap();
        db.set_session("c1", "s1", Some("claude-opus-5-20260101"))
            .unwrap();

        assert_eq!(
            db.thread("c1")
                .unwrap()
                .unwrap()
                .conversation
                .model
                .as_deref(),
            Some("claude-opus-5-20260101")
        );
        assert_eq!(db.setting(SELECTED_MODEL).unwrap().as_deref(), Some("opus"));
    }

    #[test]
    fn only_the_latest_window_per_provider_is_kept() {
        let db = db();
        let five_hour = RateLimit {
            provider_id: "claude-cli".into(),
            kind: "five_hour".into(),
            status: "allowed".into(),
            resets_at: Some(1787421000),
            using_overage: false,
            observed_at: 1787420000,
        };
        db.set_rate_limit(&five_hour).unwrap();
        db.set_rate_limit(&RateLimit {
            status: "allowed_warning".into(),
            observed_at: 1787420500,
            ..five_hour.clone()
        })
        .unwrap();
        db.set_rate_limit(&RateLimit {
            provider_id: "gemini-cli".into(),
            ..five_hour.clone()
        })
        .unwrap();

        let mut stored = db.rate_limits().unwrap();
        stored.sort_by(|a, b| a.provider_id.cmp(&b.provider_id));
        assert_eq!(stored.len(), 2);
        assert_eq!(stored[0].provider_id, "claude-cli");
        assert_eq!(stored[0].status, "allowed_warning");
        assert_eq!(stored[1].provider_id, "gemini-cli");
    }

    /// The window is a cache a run refreshes, so a row this build cannot read
    /// must not take the readable ones down with it.
    #[test]
    fn an_unreadable_window_row_is_skipped() {
        let db = db();
        db.set_setting("rate_limit:broken", Some("{oh dear"))
            .unwrap();
        db.set_rate_limit(&RateLimit {
            provider_id: "claude-cli".into(),
            kind: "five_hour".into(),
            status: "allowed".into(),
            resets_at: None,
            using_overage: false,
            observed_at: 1,
        })
        .unwrap();

        let stored = db.rate_limits().unwrap();
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].provider_id, "claude-cli");
    }

    /// Switching provider has to choose a model, and choosing the one that
    /// sorts first would throw away what the user was actually using.
    #[test]
    fn each_provider_remembers_the_model_last_asked_of_it() {
        let db = db();
        db.remember_model("claude-cli", "opus").unwrap();
        db.remember_model("opencode-cli", "opencode-go/glm-5.3")
            .unwrap();
        db.remember_model("claude-cli", "haiku").unwrap();

        let mut remembered = db.remembered_models().unwrap();
        remembered.sort();
        assert_eq!(
            remembered,
            [
                ("claude-cli".to_owned(), "haiku".to_owned()),
                ("opencode-cli".to_owned(), "opencode-go/glm-5.3".to_owned()),
            ]
        );
    }

    /// They share the settings table with the subscription windows, and neither
    /// may read the other's rows.
    #[test]
    fn a_remembered_model_is_not_mistaken_for_a_window() {
        let db = db();
        db.remember_model("claude-cli", "opus").unwrap();
        db.set_rate_limit(&RateLimit {
            provider_id: "claude-cli".into(),
            kind: "five_hour".into(),
            status: "allowed".into(),
            resets_at: None,
            using_overage: false,
            observed_at: 1,
        })
        .unwrap();

        assert_eq!(db.remembered_models().unwrap().len(), 1);
        assert_eq!(db.rate_limits().unwrap().len(), 1);
    }

    #[test]
    fn settings_round_trip_and_can_be_cleared() {
        let db = db();
        assert_eq!(db.setting(ACTIVE_CONVERSATION).unwrap(), None);
        db.set_setting(ACTIVE_CONVERSATION, Some("c1")).unwrap();
        db.set_setting(ACTIVE_CONVERSATION, Some("c2")).unwrap();
        assert_eq!(
            db.setting(ACTIVE_CONVERSATION).unwrap().as_deref(),
            Some("c2")
        );
        db.set_setting(ACTIVE_CONVERSATION, None).unwrap();
        assert_eq!(db.setting(ACTIVE_CONVERSATION).unwrap(), None);
    }

    #[test]
    fn deleting_a_conversation_takes_its_messages_with_it() {
        let db = db();
        db.ensure_conversation("c1", "hi", "claude-cli", None)
            .unwrap();
        db.add_message("c1", &message("r1:u", "user", "hi"))
            .unwrap();
        db.delete_conversation("c1").unwrap();
        assert!(db.thread("c1").unwrap().is_none());
        let orphans: i64 =
            db.0.lock()
                .unwrap()
                .query_row("SELECT count(*) FROM messages", [], |row| row.get(0))
                .unwrap();
        assert_eq!(orphans, 0);
    }

    #[test]
    fn newest_conversation_is_listed_first() {
        let db = db();
        db.ensure_conversation("c1", "older", "claude-cli", None)
            .unwrap();
        db.ensure_conversation("c2", "newer", "claude-cli", None)
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        db.add_message("c1", &message("r1:u", "user", "bump"))
            .unwrap();
        let ids: Vec<_> = db
            .list_conversations()
            .unwrap()
            .into_iter()
            .map(|c| c.id)
            .collect();
        assert_eq!(ids, ["c1", "c2"]);
    }

    #[test]
    fn truncating_keeps_the_message_it_was_given_and_everything_before_it() {
        let db = db();
        db.ensure_conversation("c1", "hi", "claude-cli", None)
            .unwrap();
        for id in ["r1:u", "r1", "r2:u", "r2"] {
            db.add_message("c1", &message(id, "user", id)).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(2));
        }

        db.truncate_after("c1", "r1").unwrap();
        let left: Vec<_> = db
            .thread("c1")
            .unwrap()
            .unwrap()
            .messages
            .into_iter()
            .map(|m| m.id)
            .collect();
        assert_eq!(left, ["r1:u", "r1"]);
    }

    // Two messages written in the same millisecond are still ordered, so a
    // truncate that fell back to `created_at` alone would take one too many.
    #[test]
    fn truncating_separates_messages_written_in_the_same_millisecond() {
        let db = db();
        db.ensure_conversation("c1", "hi", "claude-cli", None)
            .unwrap();
        for id in ["a", "b", "c"] {
            db.add_message("c1", &message(id, "user", id)).unwrap();
        }

        db.truncate_after("c1", "b").unwrap();
        let left: Vec<_> = db
            .thread("c1")
            .unwrap()
            .unwrap()
            .messages
            .into_iter()
            .map(|m| m.id)
            .collect();
        assert_eq!(left, ["a", "b"]);
    }

    #[test]
    fn a_pinned_conversation_outranks_a_newer_one() {
        let db = db();
        db.ensure_conversation("old", "older", "claude-cli", None)
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(2));
        db.ensure_conversation("new", "newer", "claude-cli", None)
            .unwrap();
        assert_eq!(listed(&db), ["new", "old"]);

        db.set_pinned("old", true).unwrap();
        assert_eq!(listed(&db), ["old", "new"]);
        assert!(db.thread("old").unwrap().unwrap().conversation.pinned);

        db.set_pinned("old", false).unwrap();
        assert_eq!(listed(&db), ["new", "old"]);
    }

    fn listed(db: &Db) -> Vec<String> {
        db.list_conversations()
            .unwrap()
            .into_iter()
            .map(|c| c.id)
            .collect()
    }

    #[test]
    fn titles_are_a_trimmed_first_line() {
        assert_eq!(title_from("  hello \n world "), "hello");
        assert_eq!(title_from("   "), "Untitled");
        assert_eq!(title_from(&"x".repeat(80)), format!("{}…", "x".repeat(64)));
    }
}
