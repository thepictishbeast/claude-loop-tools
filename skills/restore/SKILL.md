---
name: restore
description: Re-hydrate a Claude Code session from a /checkpoint. Reads ~/.claude/.checkpoint/, recreates the TaskList, resumes paused loops via /loop-resume, prints the handoff note + dirty-git-tree summary. Use as the first command after relaunching Claude Code if you ran /checkpoint before exiting.
---

# /restore — re-hydrate session state from a /checkpoint

## When to run this

First command of a new Claude Code session, after the previous
session ended with `/checkpoint`. If there's no checkpoint on disk,
this skill tells you so and stops — no harm.

## Steps

State directory: `$HOME/.claude/.checkpoint/`.

### 0. Auto-update claude-tools (added 2026-05-17)

Before rehydrating session state, refresh the toolkit itself so this
session uses the latest skills. Look for a cloned `claude-tools`
repo at these paths in order:

```
$HOME/claude-tools
$HOME/projects/claude-tools
$HOME/git/claude-tools
$HOME/code/claude-tools
/tmp/claude-tools
```

For the first hit (verify with `git remote get-url origin | grep
claude-tools`), if the working tree is clean:

```sh
cd "$REPO"
./update.sh --quiet -f
```

Behavior:
- If already up to date: silent, no output.
- If anything changed: show one-line "claude-tools updated:
  <N> commits, <M> skills" message. Continue with restore.
- If `update.sh` exits nonzero: log a one-line warning, continue
  restore with old version. Don't block rehydration on a network blip.
- If working tree is dirty: skip the update with a one-line note
  ("repo has local changes, skipped auto-update"). The user can
  manually clean up later.

After update succeeds, **note that newly-installed skills are NOT
loaded into THIS session** — Claude Code only loads skills at
session start. If the update brought in new skill names, the user
needs to relaunch Claude Code again to use them. Surface this
explicitly: "auto-updated; restart Claude Code to load new skills X, Y."

### 1. Verify a checkpoint exists

If `$HOME/.claude/.checkpoint/MANIFEST.json` is missing or
`.checkpoint/` directory is absent: tell user "no checkpoint to
restore." Stop.

### 2. Read the manifest

Parse it. Show the user a one-line summary of what's coming:

```
Restoring checkpoint from 2026-05-17T07:50:00Z:
  3 tasks · 1 paused loop · 1 dirty git tree
```

### 3. Show the handoff note

Display `$HOME/.claude/.checkpoint/handoff.md` verbatim to the user.
This is "what we were doing." Lets them sanity-check before
re-hydrating.

### 4. Re-create the tasks

For each entry in `tasks.json`, call `TaskCreate` with the original
`subject`, `description`, `activeForm`. Set status via `TaskUpdate`
to match the checkpoint state (in_progress / pending / completed).

**Note**: original task IDs will not be preserved — Claude Code
assigns new IDs. If a task had `blockedBy: ["3"]`, that "3" refers
to the old session's ID and won't resolve. Best effort: record the
old IDs as metadata, but don't try to re-link.

### 5. Resume paused loops

Invoke the `/loop-resume` skill via the Skill tool. It reads
`.paused-loops.json` (which `/checkpoint` already populated via
`/loop-pause`) and CronCreates each.

### 6. Show background-process losses

Cat `$HOME/.claude/.checkpoint/processes.txt`. If non-empty: tell
the user which background processes were running at checkpoint time
that died on exit. Ask whether they want to re-kick any.

### 7. Show dirty-git summary

Cat `$HOME/.claude/.checkpoint/git-status.txt`. If non-empty: list
the repos with uncommitted changes as a reminder.

### 8. Delete the checkpoint

Once everything's restored, **remove** `$HOME/.claude/.checkpoint/`.
This is single-shot — leaving it on disk would cause confusion if
the user runs `/restore` again later.

If any step failed (e.g. TaskCreate errored), KEEP the checkpoint
files for the failed entries so the user can retry.

### 9. Final report

```
Restored:
- 3 tasks (re-created with new IDs)
- 1 loop resumed (job ID f88a2110, every minute)

Background processes that died on exit:
- pid=12345 (cargo build) — you'd need to re-kick if still needed

Dirty git trees (still uncommitted, paul-side decision):
- /home/paul/projects/claude-tools

Pick up where you left off.
```

## Edge cases

- **No checkpoint**: report cleanly and stop. Don't error.
- **Malformed checkpoint files**: report which file is broken, leave
  the checkpoint dir in place, don't repair. User can manually fix.
- **Partial restore** (some steps succeeded, some failed): KEEP the
  checkpoint dir, report what worked + what didn't. User can manually
  fix or re-run.

## Don't

- Don't re-execute task descriptions as commands. Tasks are
  declarative descriptions of work — restoring means re-creating
  the entries, not running them.
- Don't auto-commit anything. Dirty git trees stay dirty.
- Don't try to re-launch background processes. Show them, let the
  user decide.
