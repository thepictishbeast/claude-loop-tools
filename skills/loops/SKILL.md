---
name: loops
description: Show a unified view of Claude Code cron/loop state — currently active cron jobs, currently paused loops (from /loop-pause), and recent loop history (last starts/pauses/resumes/stops). Use this to check what's running or recently ran.
---

# /loops — list active + paused + recent loop activity

Thin wrapper around `claude-loop list`. Binary owns file reads +
JSON shaping. Agent does the CronList API call.

## Steps

1. **Active jobs** — call `CronList`, render the output.

2. **Paused + recent history** — invoke the binary:

   ```sh
   claude-loop list
   ```

   Stdout is JSON with `paused` (array of paused-job records) +
   `history_tail` (last 20 lines of `loop-history.jsonl`).

3. **Render compactly**:

   ```
   Active (N):
     <id> · <cadence_human> · <prompt first 60 chars>...

   Paused (M):
     · cron=<expr> · paused at <ISO>: <prompt first 60 chars>...
     (run /loop-resume to restore)

   Recent history (latest first):
     <ISO>  <event>  <id>  <prompt first 40 chars>...
   ```

   If active + paused + history are ALL empty: "No loops active,
   paused, or in history yet. Use /loop to schedule one."

## Optional argument: `/loops history [N]`

For just the history tail:

```sh
claude-loop history -n 50
```

Default N = 20.

### Filtering by event type

Repeatable `--filter key=value` narrows the tail. Common queries:

```sh
# Only stopped loops (audit what was permanently retired):
claude-loop history --filter event=stopped

# Only paused loops (still resumable):
claude-loop history --filter event=paused

# Combined AND-narrow — paused recurring loops only:
claude-loop history --filter event=paused --filter recurring=true

# By ID (every event for one cron job):
claude-loop history --filter id=a27ed2e1
```

Filters AND together. Lines that don't parse as JSON are skipped
silently (filter mode reads the whole file because "last N matching
lines" is not the same as "last N raw lines").

## Optional argument: `/loops clear-history`

Not wired into the binary (destructive ops kept out of the
auto-callable path). Suggest `rm ~/.claude/loop-history.jsonl` if
the user really wants to truncate.

## Net visible tool calls per /loops

**2** total: `CronList` + `Bash`(claude-loop list).

## Don't

- Don't modify state files. This skill is strictly read-only.
- Don't call CronCreate / CronDelete here — those belong to
  /loop-resume / /loop-pause.
