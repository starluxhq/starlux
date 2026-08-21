# Contributing to Starlux

Thanks for your interest. This document covers how the repository is organised
and what a reviewable change looks like.

## Getting set up

See [Building from source](README.md#building-from-source) for toolchain and
per-platform prerequisites, then:

```sh
npm install
npm run tauri dev
```

## Branches

`main` is always releasable. Work happens on short-lived branches named for the
change, using the same prefixes as our commit types:

```
feat/quickbar-streaming
fix/panel-hides-on-deactivate
chore/repo-setup
docs/cli-bridge-notes
```

Open a pull request against `main`. Keep PRs scoped to one concern — a reviewer
should be able to hold the whole diff in their head.

## Commits

We use [Conventional Commits](https://www.conventionalcommits.org/), with a
short, imperative subject describing the actual change:

```
feat: add streaming NDJSON parser for claude adapter
fix: keep macOS panel visible when app deactivates
chore: add CI matrix for linux, macos and windows
docs: document Wayland global shortcut workaround
refactor: extract window setup into windows module
test: cover truncated NDJSON lines in claude adapter
```

Types in use: `feat`, `fix`, `chore`, `docs`, `refactor`, `test`, `perf`, `ci`.

## Before you open a PR

Install the repo's git hooks once after cloning:

```sh
sh scripts/install-hooks.sh
```

That points git at `.githooks/`, whose `pre-push` hook runs the same checks CI
does. To run them by hand:

```sh
npm run check                          # typecheck, build, fmt, clippy, tests
npx react-doctor@latest --scope changed
```

CI runs a build matrix across Linux, macOS, and Windows, plus React Doctor on
changed files. Note that `cargo clippy` only ever compiles *your* platform's
code — the `windows/macos.rs` and `windows/generic.rs` paths are mutually
exclusive, so a clean local run says nothing about the other two. Expect the
matrix to be the first thing that compiles your cross-platform code.

Avoid amending or force-pushing a branch that another branch has already merged
from: the merge keeps the pre-amend commit, and you end up with both the old and
new version of the change. Push a follow-up commit instead.

## Platform verification

Starlux is a windowing-heavy app, and the interesting bugs are per-platform.
If your change touches window behaviour, the CLI engine, or streaming, please
say in the PR which platforms you verified on. The checks that matter:

**macOS**
- Bar appears over a fullscreen app
- Typing in the bar does *not* deactivate the app behind it
- No Dock icon until the Workspace opens
- Switch to another app and back — the panel must still be there
  (`setHidesOnDeactivate(false)`; the default silently breaks this, and it only
  shows up in real use, never while the app is frontmost)

**Windows**
- Bar is absent from the taskbar and Alt-Tab
- Mica renders on 11, falls back cleanly on 10

**Linux**
- Renders with and without compositor blur
- No blank window or resize crash
- `starlux --toggle` shows the bar on both X11 and Wayland

## Architecture notes

- **Platform quirks live in `src-tauri/src/windows/`** and nowhere else. If a
  workaround is leaking into feature code, that's a bug in the layering.
- **Never build a shell string** for a CLI provider. Commands are assembled as
  argv arrays, and prompts go over stdin — not as arguments.
- **SQLite is the source of truth.** Both windows are thin views over the Rust
  core; state is not handed between them.
- **`rusqlite` calls run under `spawn_blocking`.** SQLite is synchronous and
  must not block the async runtime.

## Reporting bugs

Please include your OS and version, desktop environment and session type on
Linux (`echo $XDG_SESSION_TYPE`), GPU/driver if the issue is visual, and which
provider CLI was involved.
