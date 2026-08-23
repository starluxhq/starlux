use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::Value;

/// Installed and signed in are different problems with different fixes, and
/// `which` can only answer the first. A signed-out provider that reads as
/// simply absent sends the user looking for an install that is already there.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum Availability {
    Missing,
    SignedOut,
    Ready { plan: Option<String> },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Provider {
    pub id: &'static str,
    pub name: &'static str,
    pub binary: &'static str,
    /// What to run to sign in, in full. `opencode login` is not a command;
    /// `opencode auth login` is, and a launcher that guesses sends the user
    /// somewhere that does not exist.
    pub login: &'static str,
    pub availability: Availability,
    pub models: Vec<String>,
    /// Whether this provider has web tools to grant at all. With one provider
    /// the toggle could be assumed; with two it cannot.
    pub web: bool,
}

struct Entry {
    id: &'static str,
    name: &'static str,
    binary: &'static str,
    login: &'static str,
    /// Empty where the binary is the only honest source, and the models the
    /// user has depend on what they are signed in to.
    models: &'static [&'static str],
    web: bool,
}

const CATALOG: &[Entry] = &[
    Entry {
        id: "claude-cli",
        name: "Claude Code",
        binary: "claude",
        login: "claude login",
        models: &["opus", "sonnet", "haiku"],
        web: true,
    },
    Entry {
        id: "opencode-cli",
        name: "opencode",
        binary: "opencode",
        login: "opencode auth login",
        models: &[],
        web: true,
    },
];

/// Asking costs a subprocess and both windows ask on mount, but signing in from
/// a terminal should show up without restarting the app.
const TTL: Duration = Duration::from_secs(30);
const PROBE_TIMEOUT: Duration = Duration::from_secs(5);

static CACHE: Mutex<Option<(Instant, Vec<Provider>)>> = Mutex::new(None);

pub fn detect() -> Vec<Provider> {
    let mut cache = CACHE.lock().unwrap();
    if let Some((probed, providers)) = cache.as_ref() {
        if probed.elapsed() < TTL {
            return providers.clone();
        }
    }
    // Held across the probe on purpose: two windows mounting at once should
    // wait for one answer rather than each spawn their own.
    let providers = probe();
    *cache = Some((Instant::now(), providers.clone()));
    providers
}

/// Called when a run fails, so a sign-in or sign-out elsewhere is picked up on
/// the next question rather than whenever the TTL happens to lapse.
pub fn invalidate() {
    *CACHE.lock().unwrap() = None;
}

fn probe() -> Vec<Provider> {
    CATALOG.iter().map(one).collect()
}

fn one(entry: &Entry) -> Provider {
    let missing = which::which(entry.binary).is_err();
    let models: Vec<String> = if missing {
        Vec::new()
    } else {
        available_models(entry)
    };

    Provider {
        id: entry.id,
        name: entry.name,
        binary: entry.binary,
        login: entry.login,
        availability: if missing {
            Availability::Missing
        } else {
            availability(entry, &models)
        },
        models,
        web: entry.web,
    }
}

fn available_models(entry: &Entry) -> Vec<String> {
    if !entry.models.is_empty() {
        return entry
            .models
            .iter()
            .map(|model| (*model).to_owned())
            .collect();
    }
    lines(entry.binary, &["models"])
}

fn availability(entry: &Entry, models: &[String]) -> Availability {
    match entry.id {
        "claude-cli" => signed_in(entry.binary, &["auth", "status"]),
        // What it can run is what it is signed in to: an empty list is the
        // same answer `auth status` gives elsewhere, from the only question
        // this CLI answers cheaply.
        "opencode-cli" if models.is_empty() => Availability::SignedOut,
        // Nothing to ask, so the run is what finds out.
        _ => Availability::Ready { plan: None },
    }
}

/// Both `claude auth status` outcomes exit zero, so the JSON is the only signal.
fn signed_in(binary: &str, args: &[&str]) -> Availability {
    let Some(report) = ask(binary, args) else {
        // Being unable to ask is not evidence of being signed out: an older
        // build may not have the subcommand, and calling a working provider
        // broken is the worse mistake.
        return Availability::Ready { plan: None };
    };
    read_auth(&report)
}

fn read_auth(report: &Value) -> Availability {
    if report.get("loggedIn").and_then(Value::as_bool) != Some(true) {
        return Availability::SignedOut;
    }
    Availability::Ready {
        plan: report
            .get("subscriptionType")
            .and_then(Value::as_str)
            .map(str::to_owned),
    }
}

/// Plain lines out, for a CLI that answers with a list rather than JSON.
fn lines(binary: &str, args: &[&str]) -> Vec<String> {
    output(binary, args)
        .map(|out| {
            String::from_utf8_lossy(&out)
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn ask(binary: &str, args: &[&str]) -> Option<Value> {
    serde_json::from_slice(&output(binary, args)?).ok()
}

fn output(binary: &str, args: &[&str]) -> Option<Vec<u8>> {
    let mut command = Command::new(binary);
    command
        .args(args)
        // Nothing to type at, and opencode blocks forever on an open stdin: a
        // probe must never be what stalls the picker.
        .stdin(Stdio::null())
        .stderr(Stdio::null());

    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    // On its own thread so a provider that stalls — on a network check, say —
    // costs one answer rather than the whole picker.
    let (done, waiting) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = done.send(command.output());
    });

    Some(waiting.recv_timeout(PROBE_TIMEOUT).ok()?.ok()?.stdout)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn auth(json: &str) -> Availability {
        read_auth(&serde_json::from_str(json).unwrap())
    }

    #[test]
    fn a_signed_in_provider_reports_its_plan() {
        let ready = auth(
            r#"{"loggedIn":true,"authMethod":"claude.ai","apiProvider":"firstParty","subscriptionType":"max"}"#,
        );
        assert_eq!(
            ready,
            Availability::Ready {
                plan: Some("max".into())
            }
        );
    }

    #[test]
    fn a_signed_in_provider_without_a_named_plan_is_still_ready() {
        assert_eq!(
            auth(r#"{"loggedIn":true,"authMethod":"apiKey"}"#),
            Availability::Ready { plan: None }
        );
    }

    /// Installed but signed out, which is what `which` alone cannot see.
    #[test]
    fn a_signed_out_provider_is_not_a_missing_one() {
        let out = auth(r#"{"loggedIn":false,"authMethod":"none","apiProvider":"firstParty"}"#);
        assert_eq!(out, Availability::SignedOut);
        assert_ne!(out, Availability::Missing);
    }

    /// A shape we do not recognise must not be read as signed in.
    #[test]
    fn an_unreadable_report_is_signed_out() {
        assert_eq!(auth(r#"{"loggedIn":"yes"}"#), Availability::SignedOut);
        assert_eq!(auth("{}"), Availability::SignedOut);
    }
}
