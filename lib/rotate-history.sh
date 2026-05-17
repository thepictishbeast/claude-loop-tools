#!/bin/sh
# Shared helper: rotate ~/.claude/loop-history.jsonl when it exceeds 1MB.
# Sourced by skills via `. "$(dirname "$0")/../../lib/rotate-history.sh"`.
# Or invoked standalone: `./lib/rotate-history.sh [--force]`.
#
# Behavior:
#   - If history file < 1MB: no-op, exit 0
#   - If history file >= 1MB OR --force passed:
#       - Compute YYYY-MM from current date
#       - Move to ~/.claude/loop-history-YYYY-MM.jsonl
#       - Gzip the rotated file
#       - Create fresh empty history file with mode 600
#       - Append a `rotation` event to the fresh file:
#         {"event":"rotation","at":"<ISO>","archived_to":"<path>","previous_size_bytes":<N>}
#
# Race-safe via flock on ~/.claude/loop-history.lock.

set -eu

HIST_FILE="${HOME}/.claude/loop-history.jsonl"
LOCK_FILE="${HOME}/.claude/loop-history.lock"
THRESHOLD_BYTES=1048576  # 1 MB

FORCE=0
[ "${1:-}" = "--force" ] && FORCE=1

# No history file = nothing to do
[ -f "$HIST_FILE" ] || exit 0

# Acquire lock — wait up to 5 seconds for concurrent writers
exec 9>"$LOCK_FILE"
if ! flock -w 5 9; then
    echo >&2 "rotate-history: could not acquire lock at $LOCK_FILE (another writer is holding it)"
    exit 2
fi

SIZE=$(stat -c %s "$HIST_FILE" 2>/dev/null || wc -c < "$HIST_FILE")

if [ "$FORCE" -eq 0 ] && [ "$SIZE" -lt "$THRESHOLD_BYTES" ]; then
    exit 0
fi

YYYYMM=$(date -u +%Y-%m)
ARCHIVE="${HOME}/.claude/loop-history-${YYYYMM}.jsonl"

# If archive already exists (we already rotated this month), append a
# numeric suffix so we don't clobber it
n=1
while [ -e "${ARCHIVE}.gz" ] || [ -e "${ARCHIVE}" ]; do
    ARCHIVE="${HOME}/.claude/loop-history-${YYYYMM}.${n}.jsonl"
    n=$((n+1))
done

mv "$HIST_FILE" "$ARCHIVE"
gzip "$ARCHIVE"

NOW=$(date -u +%Y-%m-%dT%H:%M:%SZ)
printf '{"event":"rotation","at":"%s","archived_to":"%s.gz","previous_size_bytes":%s}\n' \
    "$NOW" "$ARCHIVE" "$SIZE" > "$HIST_FILE"
chmod 600 "$HIST_FILE"

echo "rotated $HIST_FILE → ${ARCHIVE}.gz (was ${SIZE} bytes)"
