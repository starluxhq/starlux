use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::windows;

#[cfg(target_os = "macos")]
const DEFAULT: &str = "Alt+Space";
#[cfg(not(target_os = "macos"))]
const DEFAULT: &str = "Super+Space";

/// Registration failing is the normal case on Wayland and whenever another app
/// already holds the key, so every path here logs and returns rather than
/// stopping startup: the tray and `starlux --toggle` still work without it.
pub fn register(app: &AppHandle) {
    let Some(accelerator) = wanted() else {
        return;
    };

    if let Some(reason) = unsupported() {
        eprintln!("starlux: no global hotkey — {reason}. {ADVICE}");
        return;
    }

    let shortcut = match accelerator.parse::<Shortcut>() {
        Ok(shortcut) => shortcut,
        Err(err) => {
            eprintln!("starlux: `{accelerator}` is not a valid hotkey: {err}");
            return;
        }
    };

    let registered = app
        .global_shortcut()
        .on_shortcut(shortcut, |app, _, event| {
            // Without this the bar toggles twice, once down and once up.
            if event.state == ShortcutState::Pressed {
                let _ = windows::toggle_quickbar(app);
            }
        });

    if let Err(err) = registered {
        eprintln!("starlux: could not register `{accelerator}`: {err}. {ADVICE}");
    }
}

const ADVICE: &str = "Bind `starlux --toggle` in your desktop settings instead; \
on KDE, scripts/install-kde-shortcut.sh does it for you.";

/// `STARLUX_HOTKEY` takes an accelerator like `Ctrl+Shift+Space`, or `none` for
/// anyone who binds the key in their desktop environment and does not want a
/// second registration fighting it.
fn wanted() -> Option<String> {
    let raw = std::env::var("STARLUX_HOTKEY").unwrap_or_else(|_| DEFAULT.to_owned());
    let raw = raw.trim();
    (!raw.is_empty() && !raw.eq_ignore_ascii_case("none")).then(|| raw.to_owned())
}

#[cfg(target_os = "linux")]
fn unsupported() -> Option<&'static str> {
    // Under XWayland the X11 grab works, and `STARLUX_FORCE_X11` has already
    // set this by the time registration runs.
    if std::env::var("GDK_BACKEND").as_deref() == Ok("x11") {
        return None;
    }
    (std::env::var("XDG_SESSION_TYPE").as_deref() == Ok("wayland"))
        .then_some("Wayland does not let applications grab keys")
}

#[cfg(not(target_os = "linux"))]
fn unsupported() -> Option<&'static str> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_hotkey_is_one_the_plugin_accepts() {
        assert!(
            DEFAULT.parse::<Shortcut>().is_ok(),
            "{DEFAULT} does not parse"
        );
    }
}
