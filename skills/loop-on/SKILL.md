---
name: loop-on
description: Create an event-driven loop using Monitor instead of cron. The loop fires when a watched condition becomes true (CI status changes, log line appears, file modified, PR merged, etc.) rather than on a time schedule. Use when the user says "watch for X and then do Y", "trigger when Z happens", "loop on event", or any work-shape where time-based polling is the wrong cadence.
---

# /loop-on — Monitor-driven event loop (alternative to cron)

## Why this exists

Cron loops fire on a schedule regardless of state. For "wait until
the build finishes," "watch for a PR comment," "react when a log line
appears," — cron is the wrong primitive. It either fires too often
(burning tokens on no-ops) or too rarely (latency between event and
reaction).

`Monitor` is Claude Code's event-stream primitive — each stdout line
from a watched command becomes a notification. `/loop-on` wraps
Monitor in the same lifecycle as `/loop`: registered in history,
listable via `/loops`, stoppable via `/loop-stop`, integrated with
TaskList.

## Argument parsing

```
/loop-on <condition> [then: "<prompt>"] [until: "<stop-condition>"]
/loop-on <condition> watch-cmd: "<shell-cmd>" then: "..."
```

Two ways to specify the watched condition:

### Pre-defined condition shortcuts

| Condition | What it watches | Underlying Monitor cmd |
|---|---|---|
| `pr-merged:<repo>:<pr-num>` | GitHub PR state change to merged | poll-loop using `gh api` |
| `ci-status:<repo>:<branch>` | GitHub Actions run reaches success/failure | poll-loop using `gh run list` |
| `file-changed:<path>` | inotify event on path | `inotifywait -m <path>` |
| `log-line:<path>:<pattern>` | tail -f emits matching line | `tail -F <path> \| grep --line-buffered <pattern>` |
| `cron-job-state:<id>:<new-state>` | a tracked cron transitions to specified state | poll-loop using CronList |
| `task-completed:<task-id>` | TaskList task transitions to completed | poll-loop using TaskList |
| `port-open:<host>:<port>` | TCP port becomes connectable | poll-loop using nc -z |

### Custom watch command

`watch-cmd: "<shell command>"` — any command that emits one event per
stdout line and exits when the watch should end. Same contract as
the Monitor tool.

### Stop condition

`until:` accepts:
- `"N events"` — stop after N notifications
- `"<ISO-8601>"` — stop at a specific time
- `"task-<id>-completed"` — stop when a task completes
- omitted — runs until user `/loop-stop`s, max 1 hour by Monitor timeout

## Steps

1. **Parse args.** Verify `condition` parses (either a shortcut or a
   `watch-cmd:` value present). Verify `then:` is non-empty (the prompt
   to invoke when condition fires).

2. **Translate to a Monitor invocation:**
   - For shortcuts, expand to the underlying watch command from the
     table above. Per-shortcut polling intervals: 30s for GH-API
     polls (rate-limit-friendly), 0.5s for inotify, 1s for log-line.
   - Pass `persistent: true` if the user wants the watch to run for
     the session lifetime; default `false` with timeout=3600000ms (1h).

3. **Spawn the Monitor.** This becomes the loop's runtime.

4. **Append history event:**
   ```jsonl
   {"event":"created","at":"...Z","kind":"loop-on","monitor_task_id":"<task-id-from-Monitor>","condition":"<shortcut-or-cmd>","then_prompt":"<full prompt>","until":"<stop-spec or null>","label":"<inferred>"}
   ```

5. **On each Monitor notification:**
   - Verify the loop is still active (the Monitor task exists in
     TaskList). If `stopped`, ignore residual events.
   - Invoke the `then:` prompt (via Skill if it starts with `/`,
     otherwise execute as a user message).
   - Check the `until:` condition. If satisfied: TaskStop the Monitor,
     log `stopped` event, tell user.

6. **Listing:** `/loops` lists Monitor-backed loops alongside
   cron-backed ones, distinguished by `kind: "loop-on"`. They share
   the same history log but different `TaskList` storage (Monitor
   tasks are background tasks, not TaskList items).

7. **Stopping:** `/loop-stop <monitor-task-id>` calls `TaskStop` on
   the Monitor and logs `stopped`.

## Examples

```
# React when a CI run finishes — fires once
/loop-on ci-status:thepictishbeast/PlausiDen-Loom:main then: "look at the result and post a one-line summary" until: "1 events"

# Auto-merge a PR when it goes green and reviewed
/loop-on pr-merged:thepictishbeast/PlausiDen-Loom:1 then: "verify it's actually merged, then sync local main" until: "1 events"

# Tail-watch a log for OOM
/loop-on log-line:/var/log/syslog:OOM then: "alert me and capture dmesg" until: "5 events"

# Wait for prime SSH to come back after a reboot
/loop-on port-open:plausiden-prime:22 then: "ssh prime 'systemctl status postfix'" until: "1 events"
```

## Don't

- Don't use `/loop-on` for "every N minutes do X" — that's
  /loop's domain. Time-based ≠ event-based.
- Don't compose multiple events with AND/OR inside a single
  `/loop-on`. Use multiple `/loop-on` invocations or a custom
  `watch-cmd:`.
- Don't fire the `then:` prompt at create time. First Monitor
  notification is the first fire.

## See also

- The Monitor tool — the underlying primitive
- `/loop` — time-based scheduler (cron)
- `/loop-from-task` — task-completion-driven loop (special case of
  /loop-on with `condition: task-completed:<id>`)
- `docs/LOOP_PATTERNS.md` — when to use event-driven vs cron-driven
