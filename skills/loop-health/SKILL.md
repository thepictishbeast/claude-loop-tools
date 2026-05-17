---
name: loop-health
description: Single-shot lint diagnostic across all active + paused loops and the history file. Reports missing canaries, oversized prompts, stale entries, orphan history events, malformed state files, and rotation candidates. Use when the user says "check loop health", "audit my loops", "lint my loop state", or "anything wrong with my loops?"
---

# /loop-health — lint diagnostic across all loop state

## What this checks

Single pass, no mutation. Reports findings; user decides what to fix.

### A. Active cron loops (from `CronList`)

For each:
- **A1. Untracked in history** — no `created` / `resumed` / `discovered`
  event matches this cron's ID. Suggest `/loop-track <id>` or just
  call `/loops` (which auto-discovers as a side effect).
- **A2. Missing canary** — prompt doesn't contain the "if you canceled
  or stopped this loop, you should NOT be seeing this message"
  pattern. Suggest `/loop-edit prompt-append: "Note: if you canceled
  or stopped this loop, you should NOT be seeing this message."`.
- **A3. Oversized prompt** — prompt > 800 chars. Verbose prompts make
  each fire context-heavy; cleaving steady-state instructions into
  the prompt body wastes tokens on every fire.
- **A4. Self-firing canary triggered** — if any recent history event
  shows this cron was `paused` or `stopped` AFTER its latest fire,
  but the cron still exists in CronList — the cancel didn't take.
  This is a real bug and warrants user attention.

### B. Paused loops (from `.paused-loops.json`)

For each:
- **B1. Stale paused** — `paused_at` is more than 5 days ago. The user
  probably forgot. Suggest `/loop-resume` or `/loop-stop`.
- **B2. Missing canary** — same as A2.
- **B3. Oversized prompt** — same as A3.
- **B4. `inflight_tasks` orphaned** — entry has `inflight_tasks` but
  those tasks (by recorded ID) don't exist in current `TaskList`.
  Expected when paused state predates the current session. Just a
  note, not a problem.
- **B5. Multiple paused with identical prompt** — paul merged twice.
  Suggest dedupe.

### C. History file (`loop-history.jsonl`)

- **C1. Size > 1 MB** — rotation candidate. Suggest archiving to
  `loop-history-YYYY-MM.jsonl.gz` and starting a fresh log.
- **C2. Malformed lines** — non-JSON or missing required fields
  (`event`, `at`). Report line numbers; user manually fixes.
- **C3. Future-dated events** — `at` timestamp in the future. Clock
  skew or paste error.
- **C4. Discovered-truncated events** — when `/loops` auto-discovery
  found a cron with truncated prompt and couldn't capture full text.
  Suggest manual `/loop-track <id>` with full prompt.

### D. Lock files (`.paused-loops.lock`, `loop-history.lock`)

- **D1. Stale lock** — flock file present and older than 5 minutes
  with no holder process. Suggest `rm` to clear.

### E. Auto-update freshness

- **E1. Toolkit out of date** — if the locally-cloned
  `claude-loop-tools` repo exists, check `git fetch --dry-run`
  output for incoming commits. Report count.
- **E2. Stale skill rename leftovers** — `~/.claude/skills/` contains
  a directory that's no longer in upstream's `skills/` (e.g. old
  `pause/`, `resume/`). Suggest removal.

## Output format

```
Loop health: <N> findings (<M> warnings, <K> info)

A. Active loops:
  A2 [warn] cron 04b00904: missing canary
  A4 [bug]  cron 04b00904: previously paused 2026-05-17 06:55Z but still appears in CronList

B. Paused loops: (no findings)

C. History (124 KB): (no findings)

D. Lock files: (no findings)

E. Toolkit:
  E2 [info] ~/.claude/skills/pause/ — stale rename leftover (upstream uses loop-pause)

Run /loops to see current state. No mutations were made by this check.
```

## Steps

1. Snapshot all sources: `CronList`, `~/.claude/.paused-loops.json`,
   `~/.claude/loop-history.jsonl` (last 200 lines + size), `ls
   ~/.claude/skills/`, `ls ~/.claude/*.lock`.
2. Run each check in order. Each is independent and read-only.
3. If `TaskList` available, also include B4 inflight-task check.
4. If a local clone of `claude-loop-tools` exists, `git fetch
   --dry-run` for E1.
5. Emit findings in the format above.
6. Exit summary line: "Run X to fix Y, Z to fix W" guiding the user.

## Don't

- Don't mutate state. This skill is read-only.
- Don't ask the user to confirm anything mid-run; just report and exit.
- Don't try to auto-fix findings; emit suggestions, let user decide.

## See also

- `/loops` — current state view (also auto-discovers, which is a mild
  mutation; `/loop-health` is purely read-only)
- `/loop-track` — fix A1 findings explicitly
- `/loop-edit prompt-append: …` — fix A2 / B2 missing-canary findings
