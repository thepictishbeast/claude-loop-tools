#!/bin/sh
# Install claude-loop-tools skills into the current user's
# ~/.claude/skills/ directory.
#
# Usage: ./install.sh [-f]
#   -f   overwrite existing same-named skill directories without prompt
set -eu

SKILLS_SRC="$(cd "$(dirname "$0")" && pwd)/skills"
SKILLS_DST="${HOME}/.claude/skills"

if [ ! -d "$SKILLS_SRC" ]; then
    echo >&2 "error: skills/ directory not found at $SKILLS_SRC"
    exit 1
fi

mkdir -p "$SKILLS_DST"

FORCE=0
[ "${1:-}" = "-f" ] && FORCE=1

for d in "$SKILLS_SRC"/*/; do
    skill="$(basename "$d")"
    target="$SKILLS_DST/$skill"
    if [ -e "$target" ]; then
        if [ "$FORCE" -eq 1 ]; then
            rm -rf "$target"
        else
            printf "skill '%s' already exists at %s — overwrite? [y/N] " "$skill" "$target"
            read -r ans
            case "$ans" in
                [Yy]*) rm -rf "$target" ;;
                *) echo "skipped $skill"; continue ;;
            esac
        fi
    fi
    cp -r "$d" "$target"
    echo "installed: $skill → $target"
done

echo ""
echo "Done. Restart Claude Code (or open a new session) for the skills to load."
