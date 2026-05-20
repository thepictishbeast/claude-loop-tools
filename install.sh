#!/bin/sh
# Install claude-tools: builds the `claude-loop` Rust binary
# AND copies the markdown skill specs into ~/.claude/skills/.
#
# Usage: ./install.sh [-f]
#   -f   overwrite existing same-named skill directories without prompt
set -eu

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
SKILLS_SRC="$REPO_ROOT/skills"
CRATE_DIR="$REPO_ROOT/crates/claude-loop"
SKILLS_DST="${HOME}/.claude/skills"
BIN_DST="${HOME}/.local/bin"

if [ ! -d "$SKILLS_SRC" ]; then
    echo >&2 "error: skills/ directory not found at $SKILLS_SRC"
    exit 1
fi

FORCE=0
[ "${1:-}" = "-f" ] && FORCE=1

# ───────────────────────────────────────────────────────────────
# Step 1 — build + install every Rust binary under crates/
# ───────────────────────────────────────────────────────────────
if [ -d "$REPO_ROOT/crates" ]; then
    if ! command -v cargo > /dev/null 2>&1; then
        echo >&2 "error: cargo not found in PATH. Install Rust via https://rustup.rs first."
        exit 1
    fi
    mkdir -p "$BIN_DST"
    for crate_dir in "$REPO_ROOT"/crates/*/; do
        crate_name="$(basename "$crate_dir")"
        echo "==> building $crate_name (release)…"
        cargo install --path "$crate_dir" --root "$(dirname "$BIN_DST")" --force --quiet
        echo "installed: $crate_name → $BIN_DST/$crate_name"
    done
    case ":$PATH:" in
        *":$BIN_DST:"*) ;;
        *) echo "  NOTE: $BIN_DST is not on PATH — add it to your shell rc." ;;
    esac
else
    echo "WARN: crates/ not found at $REPO_ROOT/crates — skipping binary build."
    echo "      The markdown skills below will fall back to manual orchestration."
fi

# ───────────────────────────────────────────────────────────────
# Step 2 — copy the markdown skill specs into ~/.claude/skills/
# ───────────────────────────────────────────────────────────────
mkdir -p "$SKILLS_DST"

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
