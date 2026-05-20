---
name: loop-update
description: Pull the latest claude-tools from GitHub and re-install the skills. Use when the user says "update my loop tools", "upgrade claude-tools", "pull latest loop tools", or "check for loop-tools updates". Runs `update.sh` from the cloned repo location. Tells the user what commits arrived since the last install.
---

# /loop-update — refresh claude-tools from upstream

## When to use this

The user wants to pick up the latest skills from
`github.com/thepictishbeast/claude-tools` without doing the
clone/git-pull/install.sh dance manually.

## Steps

### 1. Locate the cloned repo

The repo lives wherever the user cloned it. Try these paths in order
until one is a git repo with `origin` pointing at the right place:

```
$HOME/claude-tools
$HOME/projects/claude-tools
$HOME/git/claude-tools
$HOME/code/claude-tools
/tmp/claude-tools
```

For each candidate, verify with:

```sh
cd "$candidate" 2>/dev/null
git remote get-url origin 2>/dev/null | grep -q "claude-tools" && echo HIT
```

If none of those work, ASK the user where they cloned it. Don't
guess — wrong directory = wrong git pull.

### 2. Confirm clean working tree

```sh
cd "$REPO"
git status --porcelain
```

If output is non-empty, refuse to update — the user has local
modifications. Tell them: "your clone at $REPO has local changes; commit
or stash before updating." Don't auto-discard.

### 3. Run the update script

```sh
cd "$REPO"
./update.sh -f
```

`-f` overwrites existing same-named skills in `~/.claude/skills/`
without prompting. Safe because the upstream version is what we want.

Capture stdout — it includes:
- `git pull` result (or "already up to date")
- If changed: commits + files diff + reinstall confirmations

### 4. Detect renamed skills

If the update changed skill *names* (e.g. `pause` → `loop-pause`), the
old-named directory may still exist in `~/.claude/skills/`. Detect:

```sh
# After install, for each dir in ~/.claude/skills/ that's NOT in the
# current repo's skills/, it's stale:
for d in $HOME/.claude/skills/*/; do
    name=$(basename "$d")
    if [ ! -d "$REPO/skills/$name" ]; then
        # Only flag if it appears to be a claude-tools skill
        # (skill names matching: loops, loop-*, checkpoint, restore, pause, resume)
        case "$name" in
            loops|loop-*|checkpoint|restore|pause|resume)
                echo "stale skill: ~/.claude/skills/$name (no longer in upstream)"
                ;;
        esac
    fi
done
```

If stale skills are found: list them and ASK the user before deleting.
Don't auto-remove — they may have local customizations.

### 5. Report

Output to the user:
- Whether anything changed (commit-range summary)
- What was reinstalled
- Any stale renamed skills with the path to delete
- Reminder that new skills take effect on next Claude Code session

If nothing changed (already up to date), report briefly and stop.

## Error handling

- `git pull` reports non-fast-forward: refuse, tell user to inspect/rebase manually
- `git pull` reports network error: report and stop; don't reinstall anything
- `install.sh` returns nonzero: print its stderr, don't claim success

## Don't

- Don't `git pull --rebase` or `--force` without explicit user authorization
- Don't `git stash` user changes silently
- Don't delete stale skills without asking
- Don't run if the repo has a dirty working tree

## See also

- `update.sh` at the repo root — the actual worker; this skill wraps it
- `install.sh` — what the worker calls after the pull
- `README.md` — documents both manual update and the optional systemd-timer auto-update
