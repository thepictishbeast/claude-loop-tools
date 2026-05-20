---
name: loop-pause
description: Pause all active Claude Code cron jobs (typically /loop loops) and save their state to ~/.claude/.paused-loops.json so /loop-resume can restore them. Use this when the user wants to stop scheduled work temporarily — state is preserved, no work is lost.
---

# /loop-pause — pause active cron jobs (Rust-binary backed)

Thin wrapper around the `claude-loop pause` binary (installed at
`~/.local/bin/claude-loop`; auto-built by `install.sh` from
`crates/claude-loop/`). The binary owns all the shell ops (write
JSON, chmod, append history, lock). The agent only does the Cron*
tool calls — three visible tool calls total.

## Steps

1. **Call `CronList`** to get the active jobs.

2. **If `CronList` returns empty**: tell the user "No active cron
   jobs to pause." Stop.

2a. **Optional in-flight TaskList check**: if `TaskList` is
    available AND the user did NOT pass `--force`, call it. If any
    task has status `in_progress`, surface them and confirm before
    proceeding. If user declines, abort.

3. **Convert CronList output to JSON array** and pipe to the
   binary. One Bash call covers the JSON write + chmod + history
   append + lock acquisition:

   ```sh
   echo '[{"id":"<JOB_ID>","cron":"<CRON_EXPR>","prompt":"<FULL_PROMPT>","cadence_human":"<HUMAN>","recurring":<BOOL>,"inflight_tasks":[<TASKS>]}]' \
     | claude-loop pause
   ```

   Stdout is a JSON array of the IDs to CronDelete (one per
   captured job). Stderr on error.

   **DO NOT inline-truncate prompts.** CronList truncates at ~80
   chars — if the captured prompt is truncated, refuse the pause
   and ask the user for the full text.

4. **For each ID in the binary's stdout**: call `CronDelete`.

5. **Confirm** in one short paragraph: how many jobs paused, state
   file location, whether canary was auto-added (binary adds if
   missing), how to resume:
   `Type /loop-resume to reactivate · Edit ~/.claude/.paused-loops.json first to change interval or prompt`.

## Net visible tool calls per pause

**3** total: `CronList` + `Bash`(claude-loop pause) + `CronDelete`×N

Down from 6+ in the prior markdown-only version. Per user
feedback 2026-05-19: skills should feel atomic + save tokens.

## Don't

- Don't sleep/poll — CronDelete is immediate.
- Don't reinvent the binary's shell ops (Write/chmod/jsonl
  manually). If you find yourself doing that, you're not using
  the binary.
- Don't handle ScheduleWakeup dynamic-mode loops — they're
  per-turn, not in CronList.
