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
question has no way to reach your filesystem or the network. An empty allowlist is
not enough for this — it reads as "nothing further is pre-approved" rather than "no
tools" — and a denylist only covers the tools that existed when it was written.
Each CLI takes that instruction differently: `claude` through a session-scoped
agent on the command line, `opencode` through a configuration handed to it in its
environment, and `gemini` through a policy file written fresh for every run,
because it is the one that reads files by default when run headless.

Two kinds of grant can be given, and neither implies the other. **Working in a
folder** belongs to one conversation and starts by choosing that folder: inside it
the assistant reads and edits without asking, because a launcher has nowhere to put
an approval prompt. **Tools** are switched on in Settings and apply to the whole
app — web search and web fetch are separate switches, named individually because
the CLIs name them individually, so looking something up never costs a folder.
Whatever your own CLI settings refuse is still refused, and attaching a file grants
only that file. Both are stored in SQLite rather than in a window, so a run reads
back what was actually granted rather than what the window that asked believed —
and the Quick Bar shows both without being able to change either.

[SECURITY.md](SECURITY.md) has the rest: the artifact sandbox, the content
policies, and what to do if you find a hole in any of it.

## Accounts and credentials

Starlux does not have an account, and it does not want yours.

Each provider is the vendor's own CLI, unmodified, run exactly as you would run
it in a terminal — spawned as an argv array, never as a shell string. It authenticates the way it already does on your
machine, holds its own credential, and Starlux never reads, stores, forwards, or
proxies it — there is no key in the app, nothing in the webview, and no request
of ours that reaches a provider's servers.

That shape is deliberate rather than incidental. Some CLIs keep a bearer token
in a plain file, and lifting one to make requests ourselves would make Starlux a
credential scraper for a feature it does not need: spawning the binary gets the
same answer and leaves the secret where its owner put it.

Three things follow, and Starlux holds to all three:

- **The binary is never modified**, and never invoked in a way that disables an
  authentication method built into it. `claude --bare` is specifically not used
  — it forces `ANTHROPIC_API_KEY` and would bypass the subscription this bridge
  exists to use — and a test asserts the flag can never appear in the arguments.
- **Nothing is intermediated or resold.** One person's launcher runs on one
  person's machine against one person's account. Starlux is not a service, does
  not sit between you and a provider, and has no server of its own.
- **Provider names identify what is being run**, not what Starlux is. The model
  picker lists `Claude Code`, `Gemini CLI` and `opencode` because those are the
  binaries it spawns.

Anthropic's terms for this are at
[code.claude.com/docs/en/legal-and-compliance](https://code.claude.com/docs/en/legal-and-compliance);
each vendor's own terms govern your use of their CLI, and using your
subscription through Starlux is between you and them. Check them.

## Building from source

Requires [Rust](https://rustup.rs) (stable) and Node 20+.

```sh
pnpm install
pnpm tauri dev
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

Starlux registers **Alt+Space** — ⌥Space on macOS — on every platform. Set
`STARLUX_HOTKEY` to any accelerator (`Ctrl+Alt+Space`) to change it, or to `none`
to turn registration off.

Meta+Space is deliberately not the default: it is Spotlight on macOS and the
keyboard-layout switcher on Windows and most Linux desktops.

Registration is allowed to fail. If the key is already taken — or you are on
Wayland, which does not let applications grab keys at all — Starlux logs why and
carries on; the tray icon and the CLI still work.

That is the case to bind your desktop environment's own shortcut to:

```sh
starlux --toggle
```

On KDE, `scripts/install-kde-shortcut.sh` writes that binding for you:

```sh
./scripts/install-kde-shortcut.sh                     # Alt+Space
STARLUX_SHORTCUT="Ctrl+Alt+Space" ./scripts/install-kde-shortcut.sh
```

The key works straight away — the script registers it with the running shortcut
daemon as well as writing the config, since the daemon only re-reads that file at
login. It does not check whether the key is already in use, though: if nothing
happens, look for the conflict in System Settings and rerun with a different one.

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
Found something exploitable? [SECURITY.md](SECURITY.md) has the private
reporting channel — please use it rather than an issue.

## License

MIT — see [LICENSE](LICENSE).
