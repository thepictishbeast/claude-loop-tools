---
name: loop-edit
description: Change the interval, prompt, or both of an active /loop without going through pause+resume and without spawning a duplicate via /loop. Use this when the user says "change the interval of my loop to 5m", "update the prompt of my loop", or "edit my loop". Falls back to /loop-pause-edit-resume internally — the user gets a single-command UX.
---

# /loop-edit — change interval/prompt of a running loop in place

## Why this exists

`/loop` creates a new cron entry every time. Re-running `/loop 5m
<same prompt>` doesn't *change* the existing loop — it **duplicates** it.
`CronDelete` + `CronCreate` from scratch loses the prompt unless you
re-type it. `/loop-pause` + edit JSON + `/loop-resume` works but is
three steps.

`/loop-edit` collapses that to one command:

- `/loop-edit 5m` — change interval to 5m, keep prompt + recurring
- `/loop-edit prompt: "new text"` — change prompt, keep interval
- `/loop-edit 5m prompt: "new text"` — change both
- `/loop-edit prompt-append: "additional text"` — append to existing
  prompt (useful for adding instructions without retyping)

## Parse the argument

The user may pass any combination of:
- An interval token (`5m`, `1h`, `30s`, `2d` — same syntax as /loop)
- `prompt: "..."` — replace the prompt entirely
- `prompt-append: "..."` — append to the existing prompt (with one
  leading space)

If multiple interval tokens are passed, use the first. If both
`prompt:` and `prompt-append:` are passed, fail with a clear error
("pick one").

If NO arguments are passed, show the active loop(s) + ask the user
what to change. Don't guess.

## Interval-to-cron conversion

Same table as /loop and /loop-resume:

| Pattern             | Cron               | Notes                         |
|---------------------|--------------------|--------------------------------|
| `Nm` where N ≤ 59 | `*/N * * * *`    | every N minutes                |
| `Nm` where N ≥ 60 | `0 */H * * *`    | round to hours (H = N/60)      |
| `Nh` where N ≤ 23 | `0 */N * * *`    | every N hours                  |
| `Nd`              | `0 0 */N * *`    | every N days, midnight UTC     |
| `Ns`              | round up to 1m    | cron min granularity           |

## Steps

1. **`CronList`** to find the active loop(s).

2. **If zero active**:
   - Check `$HOME/.claude/.paused-loops.json` for paused entries.
   - If paused entry exists: tell user "no active loop, but you have
     a paused one — use /loop-resume" and stop.
   - If nothing exists: tell user "no loop to edit. Start one with
     /loop". Stop.

3. **If multiple active**: list them, ask which to edit. Don't pick
   one silently.

4. **If exactly one active**: that's the target.

5. **Capture the full prompt before deleting**:
   - From CronList output if not truncated (~80 chars).
   - **If truncated**: the same recovery path as /loop-pause —
     look for the resume file at `$HOME/.claude/.paused-loops.json`
     in case the user already paused once, OR for a session log,
     OR ask the user for the full prompt. **Refuse to edit with a
     truncated prompt** — it would silently shorten the loop.

6. **Compose the new prompt**:
   - If `prompt: "..."` → use that as the new prompt verbatim.
   - If `prompt-append: "..."` → original + " " + appended text.
   - If neither → keep the original prompt unchanged.

7. **Compose the new cron expression**:
   - If interval token → convert via the table above.
   - If no interval token → keep the original cron expression.

8. **`CronDelete <old_id>`**, then **`CronCreate`** with the new
   `cron`, `prompt`, and `recurring` (carry forward original
   recurring flag, default true).

9. **Append history event** to `$HOME/.claude/loop-history.jsonl`:
   ```jsonl
   {"event":"edited","at":"...Z","id_original":"old","id_new":"new","cron":"...","prompt_changed":true|false,"interval_changed":true|false}
   ```
   `chmod 600` on first create.

10. **Confirm** in one short paragraph: what changed, new job ID,
    new cadence, that the prompt is preserved/updated as specified.
    **Do NOT execute the prompt immediately** — `/loop-edit` is a
    config change, not a "start now" command. The next cron fire
    will run it. (Different from `/loop-resume` which DOES execute
    immediately per /loop convention.)

## Edge cases

- **Stale paused state**: if there's both an active loop AND a paused
  entry in `.paused-loops.json`, ignore the paused one — `/loop-edit`
  operates on the active loop only. Tell the user about the paused
  state so they know.
- **Interval doesn't divide cleanly**: pick the nearest clean
  interval (e.g. `90m` → `1h`) and tell the user before scheduling.
- **prompt-append: would produce a prompt > some practical limit**:
  warn but proceed. Users can /loop-edit prompt: "..." to reset.

## What this is NOT

- It's NOT for changing recurring vs one-shot — `/loop` only makes
  recurring cron jobs in this tooling.
- It's NOT for editing PAUSED loops — for those, just edit
  `.paused-loops.json` directly with `$EDITOR`. /loop-edit only
  operates on currently-active cron entries.
- It's NOT for adding a NEW loop — that's `/loop`. /loop-edit
  refuses if there's nothing to edit.

## Don't

- Don't silently truncate or modify the prompt beyond what the user
  specified.
- Don't execute the prompt immediately (would surprise the user who
  expected an interval change to only affect future fires).
- Don't write to `.paused-loops.json` — that file is for /loop-pause.
