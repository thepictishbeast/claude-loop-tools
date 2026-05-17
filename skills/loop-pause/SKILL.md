---
name: loop-pause
description: Pause all active Claude Code cron jobs (typically /loop loops) and save their state to ~/.claude/.paused-loops.json so /loop-resume can restore them. Use this when the user wants to stop scheduled work temporarily — state is preserved, no work is lost.
---

# /loop-pause — pause active cron jobs, preserve state for /loop-resume

When the user invokes this skill:

## Steps

1. **Resolve the state directory.** Use `$HOME/.claude/` (e.g.
   `/root/.claude/` if running as root, `/home/paul/.claude/` if
   running as paul). State file is `$HOME/.claude/.paused-loops.json`;
   history file is `$HOME/.claude/loop-history.jsonl`.

2. **List active cron jobs** via `CronList`.

3. **If no active jobs**: tell the user "No active cron jobs to pause."
   Don't touch the state file. Stop.

3a. **In-flight TaskList check** (added 2026-05-17): if `TaskList` is
    available in this environment, call it. If any task has status
    `in_progress`, surface them to the user BEFORE pausing:

    ```
    The following tasks are mid-flight:
      #3 [in_progress] Refactor mailer pool

    Pausing the loop won't pause your task work, but the next /loop-resume
    won't auto-resume those tasks (TaskList is session-scoped).
    Continue with pause? [y/N]
    ```

    If the user passes `--force` (e.g. `/loop-pause --force`), skip the
    confirmation. If the user declines, abort without state mutation.

    When pausing proceeds with in-flight tasks, record their snapshot
    in the saved entry's `inflight_tasks` field (see step 7) so
    /loop-resume can replay context.

4. **For each active job**, capture from the CronList output:
   - Job ID (e.g. `8dfaf6a4`)
   - Cron expression (parse from the cadence — convert "Every minute"
     to `*/1 * * * *`, "Every N hours" to `0 */N * * *`, etc.)
   - Cadence description (human-readable, e.g. "Every minute (recurring)")
   - Prompt (the colon-delimited tail). **CronList truncates to ~80
     chars.** If the prompt is truncated, you need the FULL prompt
     from another source — look for a session log (e.g.
     `~/doctrine/sessions/*-loop.md`), a resume file
     (`~/.claude/.last-loop-prompt`), or ASK the user. **Refuse to
     pause with a truncated prompt** — it would corrupt /loop-resume.

5. **Auto-canary check** (paul's pattern): the prompt should contain a
   self-check sentence so the loop body detects "if I'm seeing this
   message, the loop wasn't actually canceled". Look for any of:
   - `cancel.*should.*NOT.*see` (case-insensitive)
   - `stop.*should.*NOT.*see`
   - `if .* canceled .* you should NOT`
   - similar variants

   If absent, ADD this line to the end of the prompt before saving:
   ```
    Note: if you canceled or stopped this loop, you should NOT be seeing this message.
   ```
   Tell the user you added it. The canary protects against stale-cron
   bugs where the cron keeps firing after a logical "stop".

6. **Read existing** `~/.claude/.paused-loops.json` if it exists. If
   non-empty `[]`, ASK the user whether to merge (append) or replace.
   Default merge.

7. **Write JSON** to `~/.claude/.paused-loops.json`:
   ```json
   [
     {
       "id_original": "8dfaf6a4",
       "cron": "*/1 * * * *",
       "cadence_human": "Every minute (recurring)",
       "recurring": true,
       "prompt": "<full prompt verbatim>",
       "canary_added": true,
       "paused_at": "2026-05-17T05:40:00Z",
       "label": "optional short label - infer from prompt's first sentence",
       "inflight_tasks": [
         {"id": "3", "status": "in_progress", "subject": "Refactor mailer pool", "activeForm": "Refactoring mailer pool"}
       ]
     }
   ]
   ```
   Use Write tool with **`flock(1)`** on `~/.claude/.paused-loops.lock`
   to prevent concurrent /loop-pause runs from racing. Pseudocode:
   ```sh
   flock -w 5 ~/.claude/.paused-loops.lock -c 'write the JSON'
   ```
   Then `chmod 600` the file via Bash (prompts may contain sensitive
   context). Same flock pattern applies to the history append in step 8.

   `inflight_tasks` is OPTIONAL — only present when step 3a recorded
   tasks; absent or `[]` otherwise. Empty array = no in-flight tasks
   at pause time.

8. **Append history event** to `~/.claude/loop-history.jsonl` (create
   if missing). Format — one JSON object per line:
   ```jsonl
   {"event":"paused","at":"2026-05-17T05:40:00Z","id_original":"8dfaf6a4","cron":"*/1 * * * *","label":"..."}
   ```
   `chmod 600` the history file too.

9. **CronDelete each captured job**.

10. **Confirm** in one short paragraph: how many jobs paused, where
    state was saved, whether a canary was added, how to resume:
    `Type /loop-resume to reactivate · Edit ~/.claude/.paused-loops.json first to change interval or prompt`.

## Edge cases

- Malformed prior state: rename to `.paused-loops.json.bak`, write
  fresh, tell user.
- Non-loop cron jobs in the active list (manually CronCreate'd): pause
  them too. /loop-resume restores identically.
- File exists but is empty `[]`: treat as no prior pause.

## Don't do

- Don't sleep/poll — CronDelete is immediate.
- Don't touch loop session logs, doctrine, or other on-disk state —
  only the two skill state files.
- Don't try to handle ScheduleWakeup dynamic-mode loops (they're
  per-turn, not in CronList).
