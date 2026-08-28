use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use serde_json::Value;

use crate::engine::tools::{WEB_FETCH, WEB_SEARCH};

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

/// A model and how hard it can be asked to think. The levels are the model's
/// own, not the provider's: `opencode-go/gpt-5.6-luna` offers six and
/// `opencode-go/kimi-k3` offers one, so a ladder invented here would offer
/// levels that do not exist.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    pub id: String,
    /// Empty where the model offers no choice, and where the CLI has no flag
    /// to carry one — Gemini has neither.
    pub efforts: Vec<String>,
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
    pub models: Vec<Model>,
    /// Which of Starlux's tools this CLI has to offer. Not every provider has
    /// every one — opencode ships a fetcher and no search — so a tool granted
    /// app-wide is still only reached where it exists.
    pub tools: Vec<&'static str>,
}

struct Entry {
    id: &'static str,
    name: &'static str,
    binary: &'static str,
    login: &'static str,
    /// Empty where the binary is the only honest source, and the models the
    /// user has depend on what they are signed in to.
    models: &'static [&'static str],
    /// What every model of this provider can be asked for, where the CLI takes
    /// one flag for the whole session rather than a per-model list.
    efforts: &'static [&'static str],
    tools: &'static [&'static str],
    /// Driven over the Agent Client Protocol rather than by reading a stream of
    /// lines. Only where that buys something: opencode's `run` hands the whole
    /// answer over at once, and its ACP mode streams.
    acp: bool,
}

const CATALOG: &[Entry] = &[
    Entry {
        id: "claude-cli",
        name: "Claude Code",
        binary: "claude",
        login: "claude login",
        models: &["opus", "sonnet", "haiku"],
        // `--effort`, whose levels the CLI names in its own help and warns
        // about when they are not one of these.
        efforts: &["low", "medium", "high", "xhigh", "max"],
        tools: &[WEB_SEARCH, WEB_FETCH],
        acp: false,
    },
    Entry {
        id: "gemini-cli",
        name: "Gemini CLI",
        binary: "gemini",
        login: "gemini",
        // It has no `models` command, so this is what has been seen to answer
        // rather than what the docs imply exists: `gemini-3-pro` is a 404.
        // `auto` is the CLI's own router, and its default.
        models: &["auto", "gemini-3.5-flash", "gemini-3.1-flash-lite"],
        // It has no flag for this at all, so there is nothing to offer.
        efforts: &[],
        tools: &[WEB_SEARCH, WEB_FETCH],
        acp: false,
    },
    Entry {
        id: "opencode-cli",
        name: "opencode",
        binary: "opencode",
        login: "opencode auth login",
        models: &[],
        // Per model, and read from the binary — see `catalogued`.
        efforts: &[],
        // No search tool exists to grant, only the fetcher.
        tools: &[WEB_FETCH],
        acp: true,
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
/// Whether a provider is driven over ACP rather than by reading its stdout.
pub fn speaks_acp(provider_id: &str) -> bool {
    CATALOG
        .iter()
        .any(|entry| entry.id == provider_id && entry.acp)
}

pub fn invalidate() {
    *CACHE.lock().unwrap() = None;
}

fn probe() -> Vec<Provider> {
    CATALOG.iter().map(one).collect()
}

fn one(entry: &Entry) -> Provider {
    let missing = which::which(entry.binary).is_err();
    let models = if missing {
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
        tools: entry.tools.to_vec(),
    }
}

fn available_models(entry: &Entry) -> Vec<Model> {
    if entry.models.is_empty() {
        return described(&String::from_utf8_lossy(
            &output(entry.binary, &["models", "--verbose"]).unwrap_or_default(),
        ));
    }
    entry
        .models
        .iter()
        .map(|id| Model {
            id: (*id).to_owned(),
            efforts: entry.efforts.iter().map(|e| (*e).to_owned()).collect(),
        })
        .collect()
}

/// How hard a model can be asked to think, ranked. The names come from the
/// provider, so an unfamiliar one is kept and put last rather than dropped:
/// `opencode-go/minimax-m3` offers `none` and `thinking`, which is not a ladder
/// at all, and hiding the half we do not recognise would hide the useful half.
const LADDER: [&str; 7] = ["none", "minimal", "low", "medium", "high", "xhigh", "max"];

fn rank(effort: &str) -> usize {
    LADDER
        .iter()
        .position(|known| *known == effort)
        .unwrap_or(LADDER.len())
}

/// `opencode models --verbose` prints an id, then the pretty-printed JSON
/// describing it, over and over. Only those ids share the left margin with the
/// objects' own outer braces, which is what makes them separable without
/// reimplementing a parser — and what a compact `--verbose` would break, so the
/// fixture test is the thing that notices.
fn described(printed: &str) -> Vec<Model> {
    let json: String = printed
        .lines()
        .filter(|line| line.starts_with([' ', '{', '}']))
        .collect::<Vec<_>>()
        .join("\n");

    serde_json::Deserializer::from_str(&json)
        .into_iter::<Value>()
        .filter_map(Result::ok)
        .filter_map(|described| model_of(&described))
        .collect()
}

fn model_of(described: &Value) -> Option<Model> {
    let vendor = described.get("providerID")?.as_str()?;
    let id = described.get("id")?.as_str()?;

    // Sorted here because the CLI hands them over as a JSON object and serde
    // gives back its keys in alphabetical order, which would rank `high` below
    // `low` and `max` between them.
    let mut efforts: Vec<String> = described
        .get("variants")
        .and_then(Value::as_object)
        .map(|variants| variants.keys().cloned().collect())
        .unwrap_or_default();
    efforts.sort_by_key(|effort| rank(effort));

    Some(Model {
        id: format!("{vendor}/{id}"),
        efforts,
    })
}

fn availability(entry: &Entry, models: &[Model]) -> Availability {
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

    /// Three real models from opencode 1.18.21, kept because the shape of this
    /// output is not a promise: it drifts, and a compact `--verbose` would leave
    /// every model unreadable rather than loudly wrong.
    const MODELS: &str = include_str!("../../tests/fixtures/opencode-models.txt");

    fn efforts_of(models: &[Model], id: &str) -> Vec<String> {
        models
            .iter()
            .find(|model| model.id == id)
            .unwrap_or_else(|| panic!("{id} is in the fixture"))
            .efforts
            .clone()
    }

    #[test]
    fn reads_each_model_out_of_the_printed_descriptions() {
        let models = described(MODELS);
        let ids: Vec<&str> = models.iter().map(|model| model.id.as_str()).collect();
        assert_eq!(
            ids,
            [
                "opencode-go/glm-5.3",
                "opencode-go/minimax-m3",
                "opencode-go/qwen3.7-max"
            ]
        );
    }

    /// Alphabetically these are `high, low, max`, which reads as a ladder and
    /// is not one.
    #[test]
    fn ranks_the_levels_rather_than_alphabetising_them() {
        assert_eq!(
            efforts_of(&described(MODELS), "opencode-go/glm-5.3"),
            ["low", "high", "max"]
        );
    }

    /// Not every model offers a ladder. `thinking` is not a level we know, and
    /// dropping it would leave a switch with one position.
    #[test]
    fn keeps_a_level_it_does_not_recognise_and_puts_it_last() {
        assert_eq!(
            efforts_of(&described(MODELS), "opencode-go/minimax-m3"),
            ["none", "thinking"]
        );
    }

    #[test]
    fn a_model_that_offers_no_choice_offers_none() {
        assert!(efforts_of(&described(MODELS), "opencode-go/qwen3.7-max").is_empty());
    }

    #[test]
    fn nothing_printed_describes_nothing() {
        assert!(described("").is_empty());
        assert!(described("opencode is not signed in").is_empty());
    }

    /// Where the CLI takes one flag for the session, every model carries the
    /// same levels rather than the list being attached to the provider.
    #[test]
    fn a_catalogued_provider_hands_its_levels_to_each_model() {
        let claude = CATALOG
            .iter()
            .find(|entry| entry.id == "claude-cli")
            .unwrap();
        let models = available_models(claude);
        assert_eq!(models.len(), 3);
        assert!(models
            .iter()
            .all(|model| model.efforts == ["low", "medium", "high", "xhigh", "max"]));
    }

    /// It has no flag to carry one, so offering a level would be offering
    /// something that goes nowhere.
    #[test]
    fn gemini_offers_no_levels_because_it_has_no_flag() {
        let gemini = CATALOG
            .iter()
            .find(|entry| entry.id == "gemini-cli")
            .unwrap();
        assert!(available_models(gemini)
            .iter()
            .all(|model| model.efforts.is_empty()));
    }

    /// Only where it buys something. `opencode run` hands the whole answer over
    /// at once and its ACP mode streams; the other two stream already.
    #[test]
    fn only_opencode_is_driven_over_acp() {
        assert!(speaks_acp("opencode-cli"));
        assert!(!speaks_acp("claude-cli"));
        assert!(!speaks_acp("gemini-cli"));
        assert!(!speaks_acp("nothing-of-the-sort"));
    }

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
