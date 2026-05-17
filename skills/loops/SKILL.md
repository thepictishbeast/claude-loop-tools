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

4. **Untracked-cron discovery** (added 2026-05-17): for each active
   cron from step 1, check whether the history has any
   `created` / `resumed` / `discovered` / `paused` event with a
   matching `id` (current or `id_new`/`id_original` from earlier
   pause-resume cycles). If a cron has NO history entry, it was
   created outside this toolkit (raw `/loop`, prior session,
   inherited). Append a `discovered` event to the history file:

   ```jsonl
   {"event":"discovered","at":"<ISO-8601>","id":"<cron-id>","cron":"<expr>","cadence_human":"<human>","prompt":"<full prompt from CronList>","reason":"side-effect of /loops auto-discovery"}
   ```

   This is the ONE state mutation this skill is allowed (see "Don't"
   below — read-only EXCEPT for discovery audit log). The mutation
   makes future `/loops`, `/loop-pause`, and `/loop-edit` runs see
   prior context. Skip the append if CronList prompt is truncated
   to ~80 chars AND you cannot reconstruct full text — log a
   `discovered-truncated` event instead, with a note for the user.

5. **Inflight tasks**: if `TaskList` is available
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

- Don't modify any state files. This skill is read-only EXCEPT for
  the auto-discovery append described in step 4 — that is the single
  intentional mutation (and it's append-only audit log, not
  destructive).
- Don't call CronCreate or CronDelete. Those are /loop-resume's and
  /loop-pause's jobs.
- Don't call TaskCreate / TaskUpdate / TaskStop — read only. The
  agent decides if a task needs creating; this skill just shows
  what's there.
