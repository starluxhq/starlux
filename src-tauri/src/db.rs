use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::engine::{RateLimit, Usage};

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

/// The thread the Workspace comes back to after a restart.
pub const ACTIVE_CONVERSATION: &str = "active_conversation";
/// What the next run will ask for, which outlives any one conversation.
pub const SELECTED_PROVIDER: &str = "selected_provider";
pub const SELECTED_MODEL: &str = "selected_model";
/// One row per provider holding only its latest window. A snapshot of something
/// that moves, not a history: `messages` is where anything worth keeping goes.
const RATE_LIMIT_PREFIX: &str = "rate_limit:";

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
                error, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET
               content = excluded.content, model = excluded.model,
               tokens_in = excluded.tokens_in, tokens_out = excluded.tokens_out,
               cost_usd = excluded.cost_usd, error = excluded.error",
            params![
                message.id,
                conversation_id,
                message.role,
                message.text,
                message.model,
                message.usage.as_ref().map(|u| u.input_tokens as i64),
                message.usage.as_ref().map(|u| u.output_tokens as i64),
                message.usage.as_ref().and_then(|u| u.cost_usd),
                message.error,
                now,
            ],
        )?;
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

    pub fn agent_dir(&self, id: &str) -> rusqlite::Result<Option<String>> {
        let conn = self.0.lock().unwrap();
        conn.query_row(
            "SELECT agent_dir FROM conversations WHERE id = ?1",
            params![id],
            |row| row.get(0),
        )
        .optional()
        .map(Option::flatten)
    }

    pub fn list_conversations(&self) -> rusqlite::Result<Vec<Conversation>> {
        let conn = self.0.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, title, provider_id, provider_session_id, model, agent_dir, updated_at
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
                "SELECT id, title, provider_id, provider_session_id, model, agent_dir, updated_at
                   FROM conversations WHERE id = ?1",
                params![conversation_id],
                read_conversation,
            )
            .optional()?;

        let Some(conversation) = conversation else {
            return Ok(None);
        };

        let mut stmt = conn.prepare(
            "SELECT id, role, content, model, tokens_in, tokens_out, cost_usd, error
               FROM messages WHERE conversation_id = ?1
              ORDER BY created_at, rowid",
        )?;
        let messages = stmt
            .query_map(params![conversation_id], read_message)?
            .collect::<rusqlite::Result<Vec<_>>>()?;

        Ok(Some(Thread {
            conversation,
            messages,
        }))
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
fn migrate(conn: &mut Connection) -> rusqlite::Result<()> {
    let tx = conn.transaction()?;
    let version: i64 = tx.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version < 1 {
        tx.execute_batch(SCHEMA_V1)?;
        tx.pragma_update(None, "user_version", 1)?;
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
    })
}

fn read_message(row: &rusqlite::Row<'_>) -> rusqlite::Result<Message> {
    let input_tokens: Option<i64> = row.get(4)?;
    let output_tokens: Option<i64> = row.get(5)?;
    let cost_usd: Option<f64> = row.get(6)?;
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
            }),
        error: row.get(7)?,
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
    fn titles_are_a_trimmed_first_line() {
        assert_eq!(title_from("  hello \n world "), "hello");
        assert_eq!(title_from("   "), "Untitled");
        assert_eq!(title_from(&"x".repeat(80)), format!("{}…", "x".repeat(64)));
    }
}
