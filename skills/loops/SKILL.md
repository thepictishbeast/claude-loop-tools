---
name: loops
description: Show a unified view of Claude Code cron/loop state — currently active cron jobs, currently paused loops (from /loop-pause), and recent loop history (last starts/pauses/resumes/stops). Use this to check what's running or recently ran.
---

# /loops — list active + paused + recent loop activity

When the user invokes this skill:

## Steps

1. **Active jobs** — call `CronList`. Capture all entries.

2. **Paused state** — read `$HOME/.claude/.paused-loops.json`. If
   missing or `[]`, paused list is empty.

3. **History** — read last 20 lines (or fewer) from
   `$HOME/.claude/loop-history.jsonl`. If missing, history is empty.

4. **Inflight tasks** (added 2026-05-17): if `TaskList` is available
   in this environment, call it and capture the current task state.
   Loops without an attached task list tend to drift / restart work.
   Showing the task list inline with loop status helps the agent
   pick up exactly where it left off.

5. **Display** in this format (or similar — use whatever's clean):

   ```
   Active (N loops):
     <id> · <cadence_human> · <prompt first 60 chars>...

   Paused (N):
     · cron=<expr> · paused at <ISO>: <prompt first 60 chars>...
     (run /loop-resume to restore)

   Inflight tasks (N):
     #1 [in_progress] <subject>
     #2 [pending] <subject> (blockedBy: #1)
     #3 [completed] <subject>

   Recent history (latest first):
     <ISO>  <event>  <id>  <prompt first 40 chars>...
   ```

6. **If no active, no paused, no history**: tell user "No loops
   active, paused, or in history yet. Use /loop to schedule one."

## Optional argument: `/loops history`

If the user passes `history` as the argument, show the last 50 lines
of `loop-history.jsonl` only (skip active + paused). Useful for
auditing what's been happening over time.

## Optional argument: `/loops clear-history`

Truncate `loop-history.jsonl` to empty (after asking the user to
confirm). Don't touch active or paused state.

## Format notes

- Truncate long prompts to ~60 chars in the listing — full prompt
  lives in the JSON state file or the cron entry.
- Show ISO-8601 UTC timestamps (don't translate to local time —
  history file uses UTC and consistency matters more than locale).
- If the user has a preference set for local time, that's their tool
  (or a downstream skill) to handle.

## Don't

- Don't modify any state files. This skill is read-only.
- Don't call CronCreate or CronDelete. Those are /loop-resume's and
  /loop-pause's jobs.
- Don't call TaskCreate / TaskUpdate / TaskStop — read only. The
  agent decides if a task needs creating; this skill just shows
  what's there.
