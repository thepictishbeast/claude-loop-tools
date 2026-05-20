---
name: loop-resume
description: Resume cron jobs that were previously paused with /loop-pause. Reads ~/.claude/.paused-loops.json. Optionally takes a new interval argument (e.g. `/loop-resume 5m`) to change cadence without manually editing the JSON. Edit the JSON between pause and resume to change the prompt or other fields.
---

# /loop-resume — restore paused cron jobs (Rust-binary backed)

Thin wrapper around `claude-loop resume`. Binary owns the state
read + history append + state-file deletion. Agent does the
CronCreate API calls + executes the first iteration of each
resumed prompt.

## Argument

- `/loop-resume` — restore each job with its saved cron
- `/loop-resume 5m` / `1h` / `Nd` — apply this interval to ALL
  paused jobs (binary converts to cron internally)
- `/loop-resume 30s` — rounds up to `1m` (cron min granularity)
- If the arg doesn't parse, the binary exits non-zero with a
  diagnostic. Don't fail silently.

## Steps

1. **Invoke the binary** (one Bash call covers state-read + history
   append + state-file deletion):

   ```sh
   claude-loop resume [--interval 5m]
   ```

   Stdout is a JSON array. Each entry has fields:
   `cron`, `prompt`, `recurring`, `label`, `id_original`,
   `inflight_tasks` (possibly empty).

2. **If output is `[]`**: tell user "No paused loops to resume." Stop.

3. **For each entry in the JSON**: call `CronCreate` with `cron`,
   `prompt`, `recurring` from the entry.

4. **If any entry has non-empty `inflight_tasks`**: surface them:

   ```
   These tasks were in-flight at /loop-pause time:
     #77 [in_progress] GOAL: Improve LFI ...
     #85 [in_progress] v54 Tier-1 adoption: wire pulp ...

   TaskList is session-scoped; they aren't auto-restored. Offer to
   re-create via TaskCreate if relevant.
   ```

5. **Execute each resumed prompt now** — don't wait for the first
   cron fire. If the prompt starts with `/`, invoke via Skill;
   else act on it as user input.

6. **Confirm** in one short paragraph: how many jobs resumed, new
   CronCreate IDs, cadence (note if overridden), in-flight task
   replay status, that iter-1 ran now.

## Editing state before resume

"Hand-edit `~/.claude/.paused-loops.json` between pause and resume
to change prompt, cron, recurring, or in-flight task list before
running this skill. `/loop-resume` reads whatever's there."

## Net visible tool calls per resume

**2 + N** total: `Bash`(claude-loop resume) + `CronCreate`×N

Down from 5+ in the prior markdown-only version.
