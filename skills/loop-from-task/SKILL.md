---
name: loop-from-task
description: Wrap a TaskList task as a /loop. Creates a cron-driven loop whose prompt directs the agent to work on the specified task, and whose stop condition is the task's completion. Use when the user says "loop on task #N", "keep working on this task every X minutes until it's done", or "schedule task #N as a recurring focus".
---

# /loop-from-task — turn a TaskList item into a self-terminating loop

## Why this exists

Composing the existing primitives:

- `TaskList` is the durable cross-fire state per CLAUDE.md
- `/loop` schedules cron-driven fires
- The natural pattern "keep working on task X until it's done" needs
  glue between them — that's this skill

A loop created via `/loop-from-task` carries a self-terminating prompt:
each fire checks the task's status, continues work if `in_progress`
or `pending`, and `/loop-stop`s itself once `completed`.

## Argument parsing

```
/loop-from-task <task-id> <interval>
/loop-from-task <task-id> <interval> notes: "<additional context>"
```

- `<task-id>` — TaskList ID (e.g. `3`). Required.
- `<interval>` — same syntax as /loop (`5m`, `1h`, `30s`, `2d`).
- `notes:` — optional additional context appended to the generated
  prompt (e.g. "stop at 5pm", "PR is blocked on review").

If either positional arg is missing: show `TaskList`, ask which to
loop on and at what cadence.

## Steps

1. **Verify the task exists.** Call `TaskList`. If `<task-id>` isn't
   present, error out with the current list.

2. **Verify the task isn't already completed.** If `status == completed`,
   tell user "task #N is already done — nothing to loop on" and stop.

3. **Verify no existing loop already wraps this task.** Check
   `CronList` + `~/.claude/.paused-loops.json` for any prompt containing
   `loop-from-task: task-<id>` (we tag the generated prompt). If found,
   tell user, don't duplicate.

4. **Convert interval to cron** (same table as /loop):

   | Pattern  | Cron expression  |
   |----------|------------------|
   | `Nm` (≤59) | `*/N * * * *` |
   | `Nm` (≥60) | `0 */H * * *` (H = N/60) |
   | `Nh`     | `0 */N * * *`   |
   | `Nd`     | `0 0 */N * *`   |
   | `Ns`     | round up to 1m  |

5. **Generate the loop prompt.** Template:

   ```
   /loop-from-task: task-<id>
   
   SELF-CHECK FIRST: call TaskList. Find task #<id> ("<subject>").
   
   If task status == completed:
     → /loop-stop this loop (cron id <self-id-or-placeholder>). Then post a one-line "task #<id> done; loop terminated" and exit.
   
   If task status == in_progress:
     → Continue working on it. Don't restart. Don't fragment progress.
     
   If task status == pending:
     → TaskUpdate status=in_progress, then work on it.
   
   If task is blocked (other tasks blockedBy points to it):
     → Note the blockers in this fire's output; do NOT mark in_progress; wait.
   
   <NOTES BLOCK IF PROVIDED>
   
   Canary: if you canceled or stopped this loop, you should NOT be seeing this message.
   ```

   The cron-self-id will be substituted in step 6 after CronCreate
   returns the actual ID (use `<self>` as placeholder, then patch
   the prompt with the real ID via CronDelete + CronCreate cycle —
   OR accept that the placeholder remains and the agent uses `/loops`
   to find its own ID at fire time).

   **Simpler alternative**: don't try to embed the loop's own ID;
   the agent at fire time uses `/loops` or `CronList` to discover
   the cron carrying the matching `/loop-from-task: task-<id>` tag
   and stops THAT specific cron. This is what the skill recommends.

6. **CronCreate** with the cron expression + prompt + recurring=true.

7. **Append history event**:
   ```jsonl
   {"event":"created","at":"...Z","id":"<new-cron-id>","cron":"...","tag":"loop-from-task","task_id":"<task-id>","label":"task-<id>: <subject first 40 chars>"}
   ```

8. **TaskUpdate** the task: add metadata `loop_cron_id: <new-cron-id>`
   so future TaskList views show "this task is being looped on by
   cron X."

9. **Don't execute the prompt now.** The first cron fire (in ≤N
   minutes) is when work begins. This differs from raw /loop which
   fires immediately — for task-driven loops, we want one clean
   work-cycle window, not double-firing.

10. **Confirm** to user:
    > Loop started: cron `<id>` fires every <interval>, working on
    > task #<task-id> "<subject>" until completed. Use /loop-stop
    > <id> to cancel manually, /loops to monitor, or just mark the
    > task completed and the loop self-terminates on next fire.

## Don't

- Don't auto-mark the task `in_progress` at create time. Let the
  first cron fire decide based on task state.
- Don't refuse if the task is `pending` — that's the common case.
- Don't try to be smart about multi-task loops. One task per loop;
  user creates multiple `/loop-from-task` invocations if they want
  parallel work.
- Don't ignore the `notes:` field — it's how the user injects
  per-task constraints the generic prompt can't know.

## See also

- `/loop` — the lower-level cron primitive
- `/loop-stop` — manual cancellation (loop-from-task loops also
  self-terminate, but manual stop still works)
- `/loops` — find the cron ID currently looping on task #N
