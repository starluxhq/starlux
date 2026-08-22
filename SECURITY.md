# Security

Starlux spawns other people's binaries and renders text a language model wrote.
Both are worth being careful about, and this is what that care looks like.

## Reporting a vulnerability

Use GitHub's private reporting: **Security → Report a vulnerability** on
[starluxhq/starlux](https://github.com/starluxhq/starlux/security/advisories/new).
It opens a channel visible only to the maintainers.

Please do not open a public issue for anything exploitable. There is no bounty;
there is a fix, credit if you want it, and a note in the release.

Starlux is in early development and has no released build, so there are no
supported versions yet — `main` is the only thing to report against.

## What Starlux does with your credentials

Nothing. It never reads, stores, forwards, or proxies them. Each provider CLI
authenticates the way it already does on your machine and keeps its own secret;
Starlux spawns the binary and reads its stdout. There is no key in the app, none
in the webview, and no request of ours that reaches a provider's servers. See
[Accounts and credentials](README.md#accounts-and-credentials).

## The boundaries that matter

**A hotkey question cannot reach your filesystem.** Chat-only is the default,
and it is enforced by giving the run a session-scoped agent that declares no
tools and no MCP servers — not by an allowlist or a denylist. An empty allowlist
reads as "nothing further is pre-approved" rather than "no tools", and a
denylist only covers the tools that existed when it was written. Asked to read a
file with `Read`, `Bash` and `Glob` denied, a CLI reached it through a fourth
tool. An agent with no tools has none to reach for.

**Agent mode is a folder, chosen by you, per conversation.** Inside it the
assistant edits without asking, because a launcher has nowhere to put an
approval prompt — choosing the folder *is* the approval. Whatever your own CLI
settings refuse is still refused. The grant is stored against the conversation
in SQLite and re-read from there by the run, so a window cannot claim a
permission the database does not record.

**No command is ever built as a shell string.** Providers are invoked as argv
arrays with the prompt on stdin. There is no interpolation for a prompt to
escape from.

**Model-authored HTML runs sandboxed and offline.** Interactive answers are
served over an internal `artifact:` scheme as real documents, each with its own
`Content-Security-Policy` — `default-src 'none'`, scripts and styles inline only,
`connect-src 'none'`. An artifact can render and it can run, but nothing it was
given can leave the machine. `srcdoc` would inherit the host page's policy, so
it is deliberately not used.

**The app's own window is locked down too.** The main CSP is `default-src 'self'`
with no `connect-src`, so the frontend cannot reach any external host even if
something in it tried. Each window gets only the Tauri permissions it needs, in
`capabilities/*.json`.

## What is not a vulnerability

- A provider CLI doing something you asked it to do in agent mode, inside the
  folder you granted.
- Anything requiring an attacker who can already write to your `PATH`, your
  shell rc, or the CLI binaries themselves. At that point they do not need
  Starlux.
- The absence of code signing on unreleased builds.
