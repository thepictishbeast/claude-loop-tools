# CONTRIBUTING — for humans and AI agents alike

This repo is meant to be improved by anyone (human or AI) who finds
a real gap. The bar for a PR is "passes CI, has tests for new
behavior, doesn't expand scope beyond Claude Code cron/loop/session
state management."

## If you are an AI agent (Claude, Codex, Cursor, etc.)

You are welcome to submit improvements. Read these first:

1. **[`CLAUDE.md`](./CLAUDE.md)** — the loop-hygiene doctrine. Same
   rules apply when iterating on this repo as when participating in
   any other /loop.
2. **[`README.md`](./README.md)** — current capability surface.
3. **[`test/run-tests.sh`](./test/run-tests.sh)** — the test
   contract. Every new skill must add assertions here.

When opening a PR:

- **One concept per PR.** Don't bundle unrelated changes.
- **Add tests for new behavior.** Untested = doesn't ship.
- **Match the doctrine style.** Skill specs are Markdown frontmatter
  + a short instruction list. **Wrappers are Rust binaries** under
  `crates/<name>/` (preferred) — see `crates/claude-loop/` for
  the reference shape: clap-derived CLI, anyhow errors, serde
  state, fs2 file locks, unit tests. Pre-existing shell wrappers
  in `bin/` and `lib/` are being migrated incrementally; new tools
  start in Rust. State files stay human-readable JSON / JSONL.
  Python/Node/Go runtimes are out of scope.
- **Don't break existing skill names.** If you must rename, add a
  release note + transition period.
- **Sign your commits** with a `Co-Authored-By: ...` trailer
  identifying you as the agent + which session/host you're running
  on. Example: `Co-Authored-By: Claude Opus 4.7 (plausiden-prime) <noreply@anthropic.com>`.
  This helps the maintainer audit multi-agent contributions.

## Scope

**This repo is the umbrella for all Claude tooling** — not just
`/loop-*` (renamed from `claude-loop-tools` → `claude-tools`
2026-05-20). The `/loop-*` skills were the first tool set;
others (audit, regression-guard, file-adr, etc.) live alongside
in the same workspace.

| In scope | Out of scope |
|---|---|
| Any Claude Code skill that automates a repeated multi-step task | Anthropic API billing / account ops |
| Any MCP server scaffolded for Claude Code use | Other agent frameworks' tools |
| Cron / loop / session-state management | OS-level snapshots (use `sanoid` etc.) |
| TaskList / Monitor / checkpoint integration | Generic PM tooling |
| Multi-agent coordination protocols | Multi-agent coordination as a research field |

## How to add a new tool

1. **Confirm it's worth a skill.** Criteria: 3+ tool calls in a
   fixed sequence; clear inputs (an ID, a path, an arg); will be
   repeated 3+ times in a typical session; produces durable output.
2. **Pick a name** — short, dash-separated, matches the slash
   command (e.g. `audit-unused-dep` → `/audit-unused-dep`).
3. **Scaffold a Rust crate** at `crates/<name>/`:
   ```sh
   cargo new --bin crates/<name>
   ```
   Reference shape: `crates/claude-loop/Cargo.toml` + `src/main.rs`.
   Use clap (derive), anyhow, serde, fs2 for locks. Add unit tests
   in the same file under `#[cfg(test)] mod tests`.
4. **Write the skill spec** at `skills/<name>/SKILL.md`. Frontmatter
   + a tight Steps section that invokes the binary in ONE Bash
   call. The agent's only other tool calls should be the
   Cron*/Read/Write APIs that have no shell surface.
5. **Extend `install.sh`** to build + install your crate (look at
   how `crates/claude-loop` is handled; the pattern is one
   `cargo install --path crates/<name> --root ~/.local --force`).
6. **Add tests** to `test/run-tests.sh` for end-to-end behavior.
7. **Document under "Tools" in README.md** with a one-line
   description.

The 4 design rules every new tool follows:

1. **Atomic visible execution** — Rust binary owns multi-step shell
   ops; agent sees one Bash call per skill invocation.
2. **Token-efficient** — minimal prose in skill spec; tight code
   blocks; one summary at end (no per-step ack).
3. **Automates a frequent task** — encodes a real repeated workflow,
   not theoretical convenience.
4. **Rust, not shell** — type safety, testability, clap-derived
   self-documenting `--help`.

Pre-existing shell wrappers in `bin/` are migrating to Rust
incrementally — don't add NEW shell wrappers.

## Ideas backlog

Open work that hasn't landed yet:

- `/loop-clone` — duplicate an existing loop with a different
  schedule + prompt (useful for A/B-testing a prompt across two
  cadences).
- `/loops history --filter event=stopped` — query subcommand.
- Per-loop labels on active entries (currently labels only live
  on paused entries).
- A `/loop-tail <id>` that shows recent fires for a specific cron.
- Migration tooling for moving paused-loops state between machines.
- A Rust rewrite of the shell CLI (via PlausiDen-Forge build).

If you implement any of these, add tests and submit.

## How to test locally

```sh
cd claude-tools
cargo test --workspace          # all Rust unit tests
sh test/run-tests.sh            # end-to-end skill tests
shellcheck bin/* install.sh test/*.sh  # if you touch shell
```

CI runs the same on every push/PR. Local pre-flight saves CI
round-trips.

## Maintainer

`thepictishbeast` (paul). Reviews are first-come, first-merge for
focused PRs that pass CI and don't break the existing skills.
