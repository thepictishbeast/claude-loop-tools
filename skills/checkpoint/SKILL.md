---
name: checkpoint
description: Capture the full Claude Code session state to disk so the user can safely restart Claude Code and pick up exactly where they left off. Pauses active loops, writes TaskList + background-process snapshot + handoff notes. Use when user says "checkpoint", "save state to restart", "I want to restart Claude safely", "pause everything", "/restart". After this runs, the user can /exit + relaunch + /restore.
---

# /checkpoint — save full session state for clean restart

## Why this exists

Claude Code session state is *partly* persistent and *partly* not:

| Survives a restart? | Where it lives |
|---|---|
| ✓ Files on disk | regular filesystem |
| ✓ User memory | `~/.claude/projects/*/memory/` |
| ✓ Conversation context | only via `claude --continue` / equivalent |
| ✗ TaskList | session-scoped, dies on exit |
| ✗ Cron jobs from CronCreate | session-scoped, dies on exit |
| ✗ Background `Bash run_in_background` processes | session-scoped, dies on exit |
| ✗ Loaded skill changes since startup | reloaded fresh on next launch |
| ✗ Anything in `/tmp` not on tmpfs persistence | depends on system reboot vs Claude restart |

`/checkpoint` is a single command that drains the *not-survives*
column to disk so `/restore` (in the next session) can re-hydrate
the parts that can be rehydrated and tell you about the parts that
can't.

## Behavior summary

When you (the agent) execute `/checkpoint`:

1. **Wait for a safe stopping point.** Don't checkpoint mid-tool-call.
   If a long-running background process is mid-flight (`Bash` with
   `run_in_background: true`), explicitly mention it to the user and
   ask whether to:
   a) wait for it to finish before checkpointing
   b) leave it running (it'll die at exit) and note the loss
   c) cancel it via TaskStop and then checkpoint
   Default: ask, don't decide silently.

2. **Capture the data** (see Steps below).

3. **Report back** with a clear "safe to restart now" signal AND a
   one-line summary of what was captured + what couldn't be.

## Steps

State directory: `$HOME/.claude/.checkpoint/` (mode 700). Create if
absent. **Delete any prior checkpoint at start** — checkpoints are
single-shot, not stacked. Warn the user if they're about to overwrite
an unrestored prior checkpoint.

### 1. Tasks

Call `TaskList`, then for each task call `TaskGet` to get the full
description. Write to
`$HOME/.claude/.checkpoint/tasks.json`:

```json
[
  {
    "subject": "Build foo",
    "description": "Full description here",
    "activeForm": "Building foo",
    "status": "in_progress",
    "owner": "agent-name-or-empty",
    "blockedBy": ["3", "5"]
  },
  ...
]
```

`chmod 600`.

### 2. Active loops

Reuse `/loop-pause` (invoke via the Skill tool). It writes
`$HOME/.claude/.paused-loops.json` and CronDeletes each. The
`/checkpoint` skill doesn't duplicate that work — just delegates.

### 3. Background processes

Capture any `Bash` `run_in_background=true` tasks that are still
running. Use `TaskList` for the harness's task-runner tasks, OR
look at the conversation for recent `Bash` calls with that flag
and check their pid via `pgrep`. Write to
`$HOME/.claude/.checkpoint/processes.txt`:

```
# Background processes at checkpoint time
# (cannot be auto-restarted; user must manually re-kick if needed)
2026-05-17T07:50:00Z pid=12345 cmd="nohup cargo build --release ..." cwd=/opt/vaultwarden
2026-05-17T07:50:00Z pid=23456 cmd="nohup aideinit -y -f"             cwd=/
```

If none: write a single line `# no background processes`.

### 4. Git state

For each directory in `$HOME/projects/` (and `$HOME/code/`,
`$HOME/work/` if they exist), run `git status --porcelain` and
write any non-clean repos to
`$HOME/.checkpoint/git-status.txt`. If there are uncommitted
changes, **warn the user** in the final report. Don't commit
automatically — that's a user decision.

### 5. Handoff note

Write a short prose summary of "what was being worked on" to
`$HOME/.claude/.checkpoint/handoff.md`. Pull from:
- the in-progress tasks (priority 1)
- the last few substantive turns of the conversation (priority 2)
- any loop session log files (priority 3)

Format: ~10 lines max. Bullet form. Read like a sticky note to the
next session's agent. Example:

```markdown
# Checkpoint 2026-05-17T07:50:00Z

Last session was finishing the vaultwarden install on plausiden-prime.

In flight:
- Vaultwarden running on 127.0.0.1:8888 but Caddy vhost not hooked
  in yet (waiting on cert expansion)
- LE cert needs --expand to include vault.plausiden.com
- DNS for vault.plausiden.com is propagated

Tasks list saved: 2 in_progress, 1 pending.
Active loop: 1 paused (security/forge loop, cron */1 * * * *).

Run /restore in the new session to re-hydrate.
```

`chmod 600`.

### 6. Manifest

Write `$HOME/.claude/.checkpoint/MANIFEST.json` summarizing what
was captured (file list, timestamps, counts):

```json
{
  "checkpoint_at": "2026-05-17T07:50:00Z",
  "captured": {
    "tasks": 3,
    "active_loops_paused": 1,
    "background_processes": 0,
    "dirty_git_repos": 1
  },
  "files": [
    "tasks.json",
    "processes.txt",
    "git-status.txt",
    "handoff.md"
  ]
}
```

### 7. Tell the user

Format:

```
Checkpoint complete.

Captured:
- 3 tasks (2 in_progress, 1 pending) → ~/.claude/.checkpoint/tasks.json
- 1 active loop (security/forge, every minute) → paused via /loop-pause
- 0 background processes
- 1 dirty git tree (claude-loop-tools — uncommitted README.md edit)

Safe to /exit and relaunch Claude Code now.

After relaunch, run /restore to re-hydrate.
```

If anything was lost (a bg process that couldn't be paused, etc.),
say so explicitly with a recovery hint.

## Don't

- Don't write secret material to the checkpoint files. If a task
  description contains a secret, the checkpoint will leak it. Trust
  the user's task descriptions to already be clean.
- Don't auto-commit dirty git trees. Only show them.
- Don't try to capture the conversation history itself — that's
  Claude Code's `--continue` flag's job, not ours.
- Don't checkpoint twice without an intervening /restore. Refuse if
  an old checkpoint is still on disk (or ask to overwrite).

## What this is NOT

- It's NOT a transaction log. There's no "rewind" — just save +
  restore + (eventually) delete-on-restore.
- It's NOT a backup tool. ZFS snapshots / sanoid handle that layer.
- It's NOT an alternative to git commits — uncommitted code stays
  uncommitted; we just inventory.
