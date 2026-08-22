# Starlux

A fast, keyboard-first AI launcher for macOS, Linux, and Windows.

Press a hotkey, ask a question, get a streamed answer — then expand into a full
chat workspace when the conversation deserves one.

> **Status:** early development. The Quick Bar, CLI bridge, and Workspace are
> being built now; nothing is packaged for release yet.

## Why Starlux

Most desktop AI clients want an API key, and API tokens are billed separately
from the subscription you already pay for. Starlux can instead drive the CLI
tools you're **already logged into** — `claude`, `opencode`, and friends — as a
streaming backend. Your Claude Pro/Max or Gemini Advanced subscription becomes a
desktop assistant at no extra cost.

Two engines, one interface:

- **CLI bridge** — spawns an authenticated CLI (`claude -p --output-format
  stream-json`) and streams its output into the UI. No API key, no extra billing.
- **Direct API** — ordinary BYOK streaming against OpenAI, Anthropic, Google AI
  Studio, OpenRouter, or a local Ollama.

## Two windows

**Quick Bar** — a floating overlay on a global hotkey. Ask, read the streamed
answer, copy it, dismiss it. On macOS it's a true non-activating panel: it takes
your keystrokes without stealing focus from whatever is behind it, and it shows
over fullscreen apps.

**Workspace** — a full window with conversation history in a sidebar, multi-turn
threads, syntax-highlighted code with copy buttons, and a model switcher.
Conversations persist in SQLite and resume against the provider's own session,
so context survives restarts.

Closing the Workspace hides it rather than quitting, so the next hotkey press is
instant. A tray icon is the way back to either window, and the way out.

## Safety

CLI agents like `claude` can read, write, and execute by default. Starlux runs them
**chat-only**: the run declares no tools at all and no MCP servers, so a hotkey
question has no way to reach your filesystem. An empty allowlist is not enough for
this — it reads as "nothing further is pre-approved" rather than "no tools" — and a
denylist only covers the tools that existed when it was written.

Agent mode is opt-in per conversation and starts by choosing the folder it may work
in. That folder is the grant: inside it the assistant reads and edits without
asking, because a launcher has nowhere to put an approval prompt. Whatever your own
CLI settings refuse is still refused. The grant is stored with the conversation
rather than the window, so leaving agent mode takes effect on the next turn wherever
it is asked.

## Building from source

Requires [Rust](https://rustup.rs) (stable) and Node 20+.

```sh
npm install
npm run tauri dev
```

### Platform prerequisites

**Linux** — WebKitGTK 4.1 and its build dependencies:

```sh
# Arch
sudo pacman -S webkit2gtk-4.1 base-devel curl wget file openssl libayatana-appindicator librsvg
# Debian/Ubuntu
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev \
  libssl-dev libayatana-appindicator3-dev librsvg2-dev
```

**macOS** — Xcode Command Line Tools (`xcode-select --install`).

**Windows** — [Microsoft C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
and the WebView2 runtime (preinstalled on Windows 11).

### The global hotkey

Starlux registers **⌥Space** on macOS and **Super+Space** everywhere else. Set
`STARLUX_HOTKEY` to any accelerator (`Ctrl+Alt+Space`) to change it, or to `none`
to turn registration off.

Registration is allowed to fail. If the key is already taken — or you are on
Wayland, which does not let applications grab keys at all — Starlux logs why and
carries on; the tray icon and the CLI still work.

That is the case to bind your desktop environment's own shortcut to:

```sh
starlux --toggle
```

On KDE, `scripts/install-kde-shortcut.sh` writes that binding for you:

```sh
./scripts/install-kde-shortcut.sh                  # Meta+Space
STARLUX_SHORTCUT="Alt+Space" ./scripts/install-kde-shortcut.sh
```

The key works straight away — the script registers it with the running shortcut
daemon as well as writing the config, since the daemon only re-reads that file at
login. It does not check whether the key is already in use, though: if nothing
happens, look for the conflict in System Settings and rerun with a different one.
Meta+Space is a common launcher binding.

`starlux --toggle` works whether or not Starlux is running: a second launch hands
its arguments to the first and exits. `--workspace` and `--ask "<question>"` do
the same, which is what makes Starlux scriptable.

### Development

The Quick Bar hides as soon as it loses focus, which makes it hard to inspect or
screenshot. Run with `STARLUX_NO_BLUR_HIDE=1` to keep it on screen.

### Troubleshooting

If the window is blank or crashes on resize (most often NVIDIA + Wayland), run
with `STARLUX_SAFE_GRAPHICS=1` to disable WebKit's DMABUF renderer. See
[Tauri's Linux graphics notes](https://v2.tauri.app/develop/debug/linux-graphics/).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Issues and PRs welcome.

## License

MIT — see [LICENSE](LICENSE).
