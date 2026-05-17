#!/bin/sh
# claude-loop-tools update — git pull the repo + re-run install.sh
#
# Usage:
#   ./update.sh           # interactive — prompts before overwrite
#   ./update.sh -f        # force — overwrite without prompt
#   ./update.sh --quiet   # suppress "no changes" output (useful in cron/timer)
#
# Exit: 0 on success (whether or not anything actually changed),
#       nonzero if git pull or install fails.

set -eu

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$REPO_ROOT"

FORCE_FLAG=""
QUIET=0
for arg in "$@"; do
    case "$arg" in
        -f) FORCE_FLAG="-f" ;;
        --quiet|-q) QUIET=1 ;;
        --help|-h)
            echo "usage: $0 [-f] [--quiet]"
            echo "  -f       overwrite existing skills without prompt"
            echo "  --quiet  silent if no changes (for cron / systemd timer)"
            exit 0
            ;;
        *) echo >&2 "unknown arg: $arg"; exit 2 ;;
    esac
done

# Ensure we're in a git repo
if [ ! -d .git ]; then
    echo >&2 "error: $REPO_ROOT is not a git repo"
    exit 1
fi

# Capture old HEAD for diff reporting
OLD_HEAD="$(git rev-parse HEAD)"

# Pull. Capture output + exit status separately so we can distinguish
# "already up to date" (grep -v matches nothing → exit 1) from a real
# git failure.
PULL_OUT="$(git pull --ff-only 2>&1)"
PULL_RC=$?
if [ $PULL_RC -ne 0 ]; then
    echo "$PULL_OUT"
    echo >&2 "git pull failed (exit $PULL_RC)"
    exit 1
fi

NEW_HEAD="$(git rev-parse HEAD)"

if [ "$OLD_HEAD" = "$NEW_HEAD" ]; then
    [ "$QUIET" -eq 1 ] || echo "claude-loop-tools: already up to date ($NEW_HEAD)"
    exit 0
fi

# Print what changed
echo "claude-loop-tools updated: $OLD_HEAD → $NEW_HEAD"
echo
echo "Commits since previous install:"
git log --oneline "$OLD_HEAD".."$NEW_HEAD"
echo
echo "Files changed:"
git diff --stat "$OLD_HEAD".."$NEW_HEAD"
echo

# Re-run installer
if [ -n "$FORCE_FLAG" ]; then
    ./install.sh -f
else
    ./install.sh
fi

echo
echo "Done. New skills load on next Claude Code session."
