---
name: loop-stop
description: Permanently stop a /loop — delete the cron entry AND clear any paused state for it. Distinct from /loop-pause which preserves state for /loop-resume. Use this when the user wants to be DONE with a loop, not just pause it temporarily ("stop this loop", "cancel my loop", "kill the loop", "remove the loop").
---

# /loop-stop — permanently cancel a loop + clear all state

## When to use this vs /loop-pause

| User wants                                         | Use this skill |
|----------------------------------------------------|----------------|
| "stop temporarily, I'll resume later"              | `/loop-pause`  |
| "stop this loop, I'm done with it"                 | `/loop-stop`   |
| "cancel my loop"                                   | `/loop-stop`   |
| "kill all my loops"                                | `/loop-stop`   |
| "remove the loop"                                  | `/loop-stop`   |
| "the loop is done, clean up"                       | `/loop-stop`   |

If unsure, ASK: "do you want to pause (resumable) or stop (gone)?"

## Steps

1. **`CronList`** to find active loop(s).

2. **Read** `$HOME/.claude/.paused-loops.json` for any paused entries.

3. **If nothing active AND nothing paused**: tell user "no loop to
   stop." Don't touch any files. Done.

4. **If multiple active loops AND user passed no argument**: list
   them with IDs, ask which to stop (or "all" to stop everything).
   Don't pick silently.

5. **For each loop being stopped**:
   - Capture: `id`, `cron`, `prompt` (first 80 chars), `label` (if any)
   - **`CronDelete`** the active cron entry.
   - **Remove** the corresponding entry from
     `$HOME/.claude/.paused-loops.json` if present (match on
     `id_original` or `prompt`).
   - **Append history event** to `$HOME/.claude/loop-history.jsonl`:
     ```jsonl
     {"event":"stopped","at":"...Z","id":"...","cron":"...","label":"...","reason":"user requested"}
     ```

6. **If `.paused-loops.json` is now empty `[]`**: delete the file
   (don't leave a useless empty array on disk).

7. **Confirm** in one short paragraph: how many stopped, that
   state is gone, and that they need `/loop` (not `/loop-resume`)
   if they want to start a new one later.

## Edge cases

- **User passes a job ID as argument** (`/loop-stop 8dfaf6a4`):
  stop just that one, leave others running.
- **User passes "all"** (`/loop-stop all`): stop every active loop
  AND clear every paused entry. Confirm with summary count before
  acting if there are 3+ entries to stop.
- **Only paused entries, no active**: remove from
  `.paused-loops.json` only; nothing to CronDelete.
- **Append history even if there's no cron table entry**: a paused
  loop being stopped is still a "stopped" lifecycle event.

## What this is NOT

- It's NOT recoverable. After `/loop-stop`, the prompt is gone
  unless the user kept it elsewhere. (The history log has a
  truncated copy in the `prompt` field of older events, but that's
  for audit, not recovery.)
- It's NOT for "I want to keep the loop but stop it firing right
  now" — that's `/loop-pause`.

## Don't

- Don't silently stop everything when the user said "stop the
  loop" but multiple are active. Ask first.
- Don't leave orphaned `.paused-loops.json` entries.
- Don't append to history if `CronDelete` failed — only log
  successful stops.
