# Repository Guidelines

Starlux is a keyboard-first AI launcher: a floating Quick Bar and a Workspace,
both thin views over a Rust core that shells out to already-authenticated CLI
agents.

## Project Structure & Module Organization

- `src/` — React 19 + Vite + TypeScript frontend.
  - `windows/` one file per window (`QuickBar`, `Workspace`); window identity
    decides the view, there is no router.
  - `components/` small presentational pieces. `components/widgets/` renders the
    fenced blocks a model can emit.
  - `stores/` Zustand stores. `lib/` IPC bindings, types mirroring Rust serde,
    markdown plugins.
- `src-tauri/src/` — Rust core.
  - `windows/` per-platform window behaviour and **nothing else**.
  - `engine/` the CLI bridge: `cli.rs` spawns and streams, `adapters/` parses one
    binary each, `system_prompt.rs` is what every run is told.
  - `db.rs` SQLite, `commands.rs` the IPC surface, `platform.rs` env guards.
- `capabilities/*.json` scope Tauri permissions per window.
- Tests live beside the code: `#[cfg(test)] mod tests` in the same Rust file.

## Build, Test, and Development Commands

```sh
pnpm install
pnpm tauri dev            # app; needs the Vite dev server, which this starts
pnpm check                # typecheck, build, cargo fmt, clippy, cargo test
pnpm typecheck            # tsc --noEmit alone
pnpm dlx react-doctor@latest --scope changed
```

`cargo clippy` only ever compiles your own platform, so a clean local run says
nothing about the other two — expect the CI matrix to be the first thing that
compiles `windows/macos.rs` or `windows/generic.rs`.

## Coding Style & Naming Conventions

- 2-space indent, TypeScript throughout. Components are PascalCase `.tsx` and
  export exactly one component — a file that also exports a helper gives up Fast
  Refresh for everything in it.
- Explicit prop interfaces. Prefer composition and small pure components over
  one large one.
- Zustand for client state. **No TanStack Query** — server state here is a
  stream from a subprocess, not a cache.
- Rust: rustfmt defaults, `snake_case` modules, `cargo clippy -D warnings`.
- Comments earn their place. Write the ones that explain *why*, and leave out the
  ones that restate the line below them.

## Architecture Rules

- **Never build a shell string** for a CLI provider. Commands are argv arrays and
  prompts go over stdin.
- **Chat-only is the default.** A run declares no tools and no MCP servers, so a
  hotkey question cannot reach the filesystem. Agent mode is opt-in per
  conversation and pinned to a folder.
- **SQLite is the source of truth.** Both windows are views over the Rust core;
  state is not handed between them, and a run reads its grant back from the
  database rather than trusting the window that asked.
- **`rusqlite` runs under `spawn_blocking`.** SQLite is synchronous.
- **Platform quirks live in `src-tauri/src/windows/`.** A workaround leaking into
  feature code is a bug in the layering.

## Agent Configuration

This file is the instructions, and it is the only copy.

Skills live in `.agents/skills/<name>/`, vendor-neutral and readable by anything.
`.claude/skills/<name>` is a symlink into that directory — the form Claude Code
documents as followed — so there is one copy of each skill rather than one per
tool. Adding a skill means adding it under `.agents/` and linking it:

```sh
ln -s ../../.agents/skills/<name> .claude/skills/<name>
```

On Windows, git only materialises those symlinks with Developer Mode or
`core.symlinks=true`; without it the skill is still in `.agents/` to read, it
just is not loaded automatically.

## Commit & Pull Request Guidelines

One short conventional-commit line, no body: `feat: add a tray icon`. Branch and
open a PR per change; `main` is rebase-only, so merge with `--rebase`.

Say in the PR which platforms you verified on, and what you did **not** verify —
this is a windowing-heavy app and the interesting bugs are per-platform. See
[CONTRIBUTING.md](CONTRIBUTING.md) for the per-platform checklist.
