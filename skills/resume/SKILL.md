---
name: resume
description: Resume cron jobs that were previously paused with /pause. Reads ~/.claude/.paused-loops.json. Optionally takes a new interval argument (e.g. `/resume 5m`) to change cadence without manually editing the JSON. Edit the JSON between pause and resume to change the prompt or other fields.
---

# /resume — restore paused cron jobs from on-disk state

## Parse the argument

The user may pass an optional interval to override the saved cadence:
- `/resume` — restore each paused job with its original cron
- `/resume 5m` / `/resume 1h` / `/resume Nd` — restore but change ALL
  paused jobs to that interval
- `/resume 30s` — rounds up to `1m` (cron min granularity); tell user

If the argument doesn't parse as an interval, ignore it and proceed
with the saved cadences. Don't fail.

## Interval-to-cron conversion (same as /loop)

| Pattern             | Cron               | Notes                          |
|---------------------|--------------------|---------------------------------|
| `Nm` where N ≤ 59 | `*/N * * * *`    | every N minutes                 |
| `Nm` where N ≥ 60 | `0 */H * * *`    | round to hours (H = N/60)       |
| `Nh` where N ≤ 23 | `0 */N * * *`    | every N hours                   |
| `Nd`              | `0 0 */N * *`    | every N days, midnight UTC      |
| `Ns`              | round up to 1m    | cron min granularity            |

If the interval doesn't cleanly divide its unit, pick the nearest
clean one and tell the user before scheduling.

## Steps

1. **Resolve the state directory** — `$HOME/.claude/`.

2. **Read** `$HOME/.claude/.paused-loops.json`.

3. **Missing or empty `[]`**: tell user "No paused loops to resume."
   Stop. Don't delete the file.

4. **Malformed JSON**: tell user, show the path, don't try to repair.

5. **For each entry**:
   - Determine cron expression: use the arg-override if provided,
     else the entry's `cron` field.
   - Call `CronCreate` with `cron`, `prompt` (verbatim from the
     entry), `recurring` (from entry, default true).
   - Capture the new job ID.

6. **Append history event** to `$HOME/.claude/loop-history.jsonl`:
   ```jsonl
   {"event":"resumed","at":"...Z","id_original":"old","id_new":"new","cron":"...","interval_override":"5m or null"}
   ```

7. **Delete** `$HOME/.claude/.paused-loops.json` (state consumed).
   Idempotent — if any CronCreate failed, KEEP the failed entries in
   the file instead and report which.

8. **Execute the prompt now** for each resumed loop, per /loop
   convention — don't wait for the first cron fire. If the prompt
   starts with `/`, invoke via Skill; else act on it as user input.

9. **Confirm** in one short paragraph: how many resumed, new IDs,
   cadence (note if overridden), that iter-1 is running now.

## Editing the JSON before /resume

Tell users in the confirmation: "Hand-edit
`$HOME/.claude/.paused-loops.json` between pause and resume to change
the prompt, cron, or recurring flag. /resume reads whatever's there."

## Edge cases

- Argument doesn't parse as interval — ignore, use saved cadences.
- One CronCreate fails — continue with others, rewrite JSON keeping
  only failed entries, report.
- Multiple entries with same prompt (rare, if pause merged previously)
  — restore both; user can CronDelete duplicates if undesired.

## Don't

- Don't silently modify prompts. Argument override applies to interval
  only.
- Don't try to deduplicate against currently-active jobs (resume is
  authoritative; user can clean up after).
