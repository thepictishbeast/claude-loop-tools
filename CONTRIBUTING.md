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
- **Match the doctrine style.** Skills are POSIX shell + Markdown.
  No Python/Node/Go runtimes for the wrapper. State files stay
  human-readable JSON / JSONL.
- **Don't break existing skill names.** If you must rename, add a
  release note + transition period.
- **Sign your commits** with a `Co-Authored-By: ...` trailer
  identifying you as the agent + which session/host you're running
  on. Example: `Co-Authored-By: Claude Opus 4.7 (plausiden-prime) <noreply@anthropic.com>`.
  This helps the maintainer audit multi-agent contributions.

## Scope

| In scope | Out of scope |
|---|---|
| Claude Code cron jobs (CronCreate / Delete / List) | Anthropic API account / billing |
| `/loop` lifecycle (pause / resume / edit / stop / status) | Other agent frameworks' loops |
| Session state checkpoint / restore | OS-level snapshots (use `sanoid` etc.) |
| TaskList integration with loops | Generic project management tooling |
| Multi-agent coordination protocols around loops | Multi-agent coordination in general |

Out-of-scope ideas don't belong here. Open a separate repo and
optionally link it from this README.

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
cd claude-loop-tools
sh test/run-tests.sh
shellcheck bin/* install.sh test/*.sh  # if you touch shell
```

CI runs the same on every push/PR. Local pre-flight saves CI
round-trips.

## Maintainer

`thepictishbeast` (paul). Reviews are first-come, first-merge for
focused PRs that pass CI and don't break the existing skills.
