# claude-loop-tools

[![test](https://github.com/thepictishbeast/claude-loop-tools/actions/workflows/test.yml/badge.svg)](https://github.com/thepictishbeast/claude-loop-tools/actions/workflows/test.yml)

Pause, resume, edit, and audit Claude Code cron jobs (e.g. `/loop`).

Claude Code ships with `/loop` but no way to pause a long-running
loop without losing it, change its interval without re-typing the
prompt, or audit what's been scheduled over time. This adds five
skills:

- **`/loop-pause`** — pause all active cron jobs. State is written to
  `~/.claude/.paused-loops.json` and the cron entries deleted.
  Nothing is lost.
- **`/loop-resume`** — restore the paused jobs. Optional arg changes the
  interval inline (`/loop-resume 5m`). Edit the JSON between pause and
  resume to change the prompt, cron, or any other field.
- **`/loop-edit`** — change the interval, prompt, or both of an
  **already-running** loop without going through pause+resume.
  Forms: `/loop-edit 5m`, `/loop-edit prompt: "new text"`,
  `/loop-edit 5m prompt: "..."`, `/loop-edit prompt-append: "..."`.
- **`/loop-stop`** — permanently cancel a loop. Deletes the cron AND
  clears any paused state. Distinct from `/loop-pause` (which is
  resumable). Use for "I'm done with this loop, clean up."
- **`/loops`** — show a unified view of active + paused + recent
  history (last 20 events).
- **`/checkpoint`** — save FULL session state (tasks + active loops
  + background processes + dirty git trees + handoff note) to
  `~/.claude/.checkpoint/`. Use before you `/exit` so you can pick up
  exactly where you left off.
- **`/restore`** — first command of a new session. Reads the
  `~/.claude/.checkpoint/` state, re-creates the TaskList, resumes
  paused loops, shows the handoff note, then deletes the checkpoint.

History is appended to `~/.claude/loop-history.jsonl` on every
pause/resume/edit/stop — append-only, one JSON line per event.

## Patterns (general-purpose loop design)

See [`docs/LOOP_PATTERNS.md`](./docs/LOOP_PATTERNS.md) for the
catalogue covering:

- **Stop conditions** — empty task list / max iterations / error
  budget exhausted / deadline / success condition met / drift
  detection / external signal / composed-with-OR semantics.
- **Interval strategies** — fixed cron / dynamic mode (no
  interval token, self-paced via `ScheduleWakeup`) / adaptive
  cron (agent retunes its own cadence via `/loop-edit` based on
  observed state) / exponential backoff on idle / time-windowed
  (work-hours fast, night slow).
- **General-purpose loop design checklist + prompt template** —
  every decision (cadence shape / interval / stop conditions /
  scope / reporting / recovery) explicit before scheduling.

This toolkit deliberately keeps the skills low-level (`/loop`
schedules, `/loop-edit` retunes, `/loop-stop` cancels). The
PATTERNS doc shows how to compose them into loops that fit
arbitrary work shapes — bursty / steady / decreasing /
event-driven — without retyping verbose prompts.

## Install

```sh
git clone https://github.com/thepictishbeast/claude-loop-tools
cd claude-loop-tools
mkdir -p ~/.claude/skills
cp -r skills/* ~/.claude/skills/
```

Restart Claude Code (or open a new session) so the skills are
discovered.

## Usage

```
/loop 1m do-some-recurring-task         # start a loop
…
/loops                                  # what's active/paused/recent

# Modify in place (no need for pause+resume cycle):
/loop-edit 5m                           # change interval, keep prompt
/loop-edit prompt: "new task text"      # change prompt, keep interval
/loop-edit 5m prompt: "..."             # change both
/loop-edit prompt-append: "and CC me"   # tack onto existing prompt

# Pause/resume cycle (state preserved between):
/loop-pause                             # state saved, loop cancelled
/loop-resume                            # restore exactly as paused
/loop-resume 5m                         # restore with new cadence
$EDITOR ~/.claude/.paused-loops.json    # hand-edit anything before resume

# Permanent stop:
/loop-stop                              # delete cron + clear state

# Full session restart (paul wants to relaunch Claude Code):
/checkpoint                             # save tasks, loops, bg procs, notes
# then /exit + relaunch Claude Code (use --continue for chat history)
/restore                                # first command in new session
```

## Editing the prompt or cron of a paused loop

The state file is a plain JSON array; edit with any text editor:

```sh
$EDITOR ~/.claude/.paused-loops.json
```

Each entry has:

| field           | purpose                                              |
|-----------------|------------------------------------------------------|
| `cron`          | cron expression (e.g. `*/5 * * * *`)                 |
| `cadence_human` | human-readable cadence (informational only)          |
| `recurring`     | boolean — does the cron auto-renew?                  |
| `prompt`        | the verbatim message Claude receives each fire       |
| `canary_added`  | whether `/loop-pause` auto-added a canary line            |
| `paused_at`     | ISO-8601 UTC timestamp                               |
| `label`         | optional short label (informational only)            |
| `id_original`   | the original job ID from before pause (informational)|

Save and `/loop-resume` — the new values take effect.

## The canary

`/loop-pause` auto-appends a self-check sentence to the prompt if one is
not present:

> Note: if you canceled or stopped this loop, you should NOT be
> seeing this message.

This catches the bug where the cron keeps firing after the loop
logic has reached its "done" condition — the canary in the message
tells the agent to stop and clean up.

Disable canary auto-add by editing the skill's SKILL.md (find the
"Auto-canary check" section and remove it), or set the saved entry's
`canary_added` field to `false` and remove the sentence by hand from
`prompt`.

## State files

| File                                | Mode | Purpose                          |
|-------------------------------------|------|----------------------------------|
| `~/.claude/.paused-loops.json`      | 600  | Currently-paused loops           |
| `~/.claude/loop-history.jsonl`      | 600  | Append-only audit history        |

The mode-600 default is because prompts may contain sensitive
context. The skills `chmod 600` after writing.

## Design constraints

- Cron entries created with `CronCreate` are **session-scoped** —
  they die when the Claude Code session ends. The state file
  persists, so `/loop-resume` works even across sessions.
- `/loop` dynamic mode (no interval, uses `ScheduleWakeup`) isn't
  cron-backed and isn't visible to these tools. Pausing a
  dynamic-mode loop is "stop replying" — which happens automatically
  when the user doesn't interact.
- These skills don't try to deduplicate or validate cron logic.
  They're a thin layer over `CronCreate` / `CronDelete` / `CronList`.

## Loop hygiene (for AI agents)

When `/loop` fires every minute, the same prompt is re-injected each
time. Without explicit hygiene, agents tend to:

- Restart finished work every iteration
- Drop mid-task work to "act on" the re-injected prompt
- Lose track of what's in-flight vs. queued vs. done
- Bloat the iteration log with duplicate state

**[`CLAUDE.md`](./CLAUDE.md) is the contract every loop participant
reads first.** Key rules it codifies:

1. A loop fire is a SIGNAL to continue, not a command to start
   something new. Finish in-flight work first.
2. Maintain an explicit task list (`TaskCreate` / `TaskUpdate`).
   The task list is the durable state that survives between fires;
   the loop prompt itself doesn't carry state.
3. Mid-iter user messages get acked + tracked as tasks, not allowed
   to interrupt the current tool call.
4. Tight-tick fires should produce a one-line health check and stop —
   don't append redundant log entries for "still healthy".
5. Substantive fires do work fully — don't fragment a 5-minute task
   across 5 separate iterations.

Recommended workflow:

```
/loop 5m work on the tasks in my TaskList; create new ones for
follow-ups; log only when state changes
```

vs. the failure mode:

```
/loop 1m do everything you can       # too aggressive — every fire
                                       # tries to start fresh work
```

## License

MIT.
