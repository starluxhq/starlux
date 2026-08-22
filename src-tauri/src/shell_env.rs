//! The login shell's `PATH`, imported once at startup.
//!
//! A GUI process inherits the system environment, not the one a user's shell rc
//! builds. Launched from Finder, the Dock or a `.desktop` entry, Starlux cannot
//! see a CLI installed in `~/.local/bin`, a version manager's directory or
//! Homebrew — so every provider reads as missing in a packaged build while
//! working perfectly under `tauri dev`, where the terminal supplies the
//! environment.

use std::collections::{HashMap, HashSet};
use std::ffi::OsString;
use std::path::PathBuf;

/// Widens the process `PATH` to what an interactive login shell would see.
/// Every failure path leaves the inherited `PATH` untouched: a wrong `PATH` is
/// worse than a narrow one.
pub fn import() {
    // Launched from a terminal, so the environment we already have is the one
    // the user's shell built.
    if std::env::var_os("TERM").is_some() {
        return;
    }

    let Some(dump) = capture() else {
        return;
    };
    let Some(captured) = parse_env0(&dump).remove("PATH") else {
        return;
    };

    let current = std::env::var("PATH").unwrap_or_default();
    if let Some(merged) = merge_path(&captured, &current) {
        std::env::set_var("PATH", merged);
    }
}

/// `env -0` writes NUL-separated `KEY=VALUE`, which is what makes it safe to
/// parse: a value may legally contain newlines.
fn parse_env0(dump: &[u8]) -> HashMap<String, String> {
    dump.split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
        .filter_map(|entry| {
            let (key, value) = std::str::from_utf8(entry).ok()?.split_once('=')?;
            Some((key.to_owned(), value.to_owned()))
        })
        .collect()
}

/// The shell's ordering wins, with anything only the process knew about kept
/// behind it. `None` when the result would not differ from what we already have.
fn merge_path(captured: &str, current: &str) -> Option<OsString> {
    let mut seen = HashSet::new();
    let mut entries: Vec<PathBuf> = Vec::new();

    for entry in captured.split(':').chain(current.split(':')) {
        // An empty entry means the working directory, which is a foothold for
        // anything that can write where Starlux happens to be run from.
        if entry.is_empty() || !seen.insert(entry) {
            continue;
        }
        // A NUL makes `set_var` panic, so one unusable directory would
        // otherwise cost us the whole capture.
        if entry.contains('\0') {
            continue;
        }
        entries.push(PathBuf::from(entry));
    }

    let merged = std::env::join_paths(entries).ok()?;
    (merged != *current).then_some(merged)
}

#[cfg(not(unix))]
fn capture() -> Option<Vec<u8>> {
    None
}

#[cfg(unix)]
fn capture() -> Option<Vec<u8>> {
    use std::io::Write;
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};
    use std::time::Duration;

    // Prompt plugins can block on input that will never come.
    const TIMEOUT: Duration = Duration::from_secs(5);

    let shell = std::env::var("SHELL").ok()?;
    let dump = std::env::temp_dir().join(format!("starlux-env-{}", std::process::id()));
    let target = dump.to_str()?;
    // The path is interpolated into a shell command, and a quote in it would
    // close the string early.
    if target.contains('\'') {
        return None;
    }

    let mut child = Command::new(shell)
        // No `-s`: POSIX shells read a piped stdin as the script anyway, and
        // fish rejects the flag outright.
        .args(["-i", "-l"])
        // Our own environment would otherwise show up in the capture as if the
        // shell had set it.
        .env_clear()
        .envs(inherited())
        .stdin(Stdio::piped())
        // An interactive login shell greets stdout with banners, motd and prompt
        // escapes, so the dump goes to a file instead.
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        // Keeps a shell that reaches for terminal control from stopping us.
        .process_group(0)
        .spawn()
        .ok()?;

    if let Some(mut stdin) = child.stdin.take() {
        let _ = writeln!(stdin, "env -0 > '{target}'");
    }

    let finished = wait_for(&mut child, TIMEOUT);
    if !finished {
        let _ = child.kill();
        let _ = child.wait();
    }

    let bytes = finished.then(|| std::fs::read(&dump).ok()).flatten();
    let _ = std::fs::remove_file(&dump);
    bytes
}

#[cfg(unix)]
fn inherited() -> Vec<(String, String)> {
    ["HOME", "USER", "SHELL"]
        .iter()
        .filter_map(|key| Some(((*key).to_owned(), std::env::var(key).ok()?)))
        .collect()
}

#[cfg(unix)]
fn wait_for(child: &mut std::process::Child, limit: std::time::Duration) -> bool {
    let deadline = std::time::Instant::now() + limit;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => return true,
            Ok(None) if std::time::Instant::now() < deadline => {
                std::thread::sleep(std::time::Duration::from_millis(25))
            }
            _ => return false,
        }
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;

    #[test]
    fn reads_a_value_that_spans_lines() {
        let dump = b"PATH=/opt/bin:/usr/bin\0GREETING=one\ntwo\0EMPTY=\0";
        let env = parse_env0(dump);

        assert_eq!(env["PATH"], "/opt/bin:/usr/bin");
        assert_eq!(env["GREETING"], "one\ntwo");
        assert_eq!(env["EMPTY"], "");
    }

    #[test]
    fn ignores_entries_that_are_not_assignments() {
        let env = parse_env0(b"\0not an assignment\0OK=yes\0");
        assert_eq!(env.len(), 1);
        assert_eq!(env["OK"], "yes");
    }

    #[test]
    fn the_shell_leads_and_nothing_is_lost_or_repeated() {
        let merged = merge_path("/home/me/.local/bin:/usr/bin", "/usr/bin:/sbin").unwrap();
        assert_eq!(merged, "/home/me/.local/bin:/usr/bin:/sbin");
    }

    #[test]
    fn one_unusable_directory_does_not_cost_the_rest() {
        let merged = merge_path("/home/me/.local/bin:/bad\0dir", "/usr/bin").unwrap();
        assert_eq!(merged, "/home/me/.local/bin:/usr/bin");
    }

    #[test]
    fn the_working_directory_never_joins_the_path() {
        let merged = merge_path("/opt/bin::", "/usr/bin:").unwrap();
        assert_eq!(merged, "/opt/bin:/usr/bin");
    }

    #[test]
    fn a_capture_that_adds_nothing_leaves_the_path_alone() {
        assert!(merge_path("", "/usr/bin:/sbin").is_none());
        assert!(merge_path("/usr/bin:/sbin", "/usr/bin:/sbin").is_none());
    }
}
