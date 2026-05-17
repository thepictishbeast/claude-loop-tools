#!/bin/sh
# Local test runner. Validates SKILL.md frontmatter, install.sh,
# README structure. Runs from the repo root or test/ directory.
#
# Usage: ./test/run-tests.sh
# Exit: 0 if all tests pass, nonzero otherwise.

set -eu

# Resolve repo root
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
fail() { echo "  FAIL: $*"; FAIL=$((FAIL+1)); }
pass() { echo "  pass: $*"; PASS=$((PASS+1)); }

# ----------------------------------------------------------------------
echo "[1] SKILL.md frontmatter validation"

for f in skills/*/SKILL.md; do
    skill="$(basename "$(dirname "$f")")"

    # Must start with --- (frontmatter open)
    if ! head -1 "$f" | grep -qx '\-\-\-'; then
        fail "$skill: SKILL.md does not start with ---"
        continue
    fi

    # Must have a closing ---
    if ! awk 'NR==1 && /^---$/{c++; next} c==1 && /^---$/{c++; print NR; exit}' "$f" | grep -q .; then
        fail "$skill: SKILL.md missing closing ---"
        continue
    fi

    # Extract frontmatter
    fm="$(awk '/^---$/{c++; if(c==2)exit} c==1' "$f")"

    # Must have name field
    if ! echo "$fm" | grep -qE '^name: '; then
        fail "$skill: missing 'name:' field"
        continue
    fi

    # Must have description field
    if ! echo "$fm" | grep -qE '^description: '; then
        fail "$skill: missing 'description:' field"
        continue
    fi

    # name should match directory
    declared_name="$(echo "$fm" | awk -F': ' '/^name: /{print $2; exit}')"
    if [ "$declared_name" != "$skill" ]; then
        fail "$skill: frontmatter name '$declared_name' does not match directory '$skill'"
        continue
    fi

    # description should be non-empty
    desc="$(echo "$fm" | awk -F': ' '/^description: /{print $2; exit}')"
    if [ -z "$desc" ]; then
        fail "$skill: description is empty"
        continue
    fi

    pass "$skill: frontmatter valid (name=$skill, description present)"
done

# ----------------------------------------------------------------------
echo ""
echo "[2] install.sh syntax check"

if sh -n install.sh; then
    pass "install.sh: syntax OK"
else
    fail "install.sh: syntax error"
fi

# ----------------------------------------------------------------------
echo ""
echo "[3] install.sh dry-run into temp HOME"

TESTHOME="$(mktemp -d)"
trap 'rm -rf "$TESTHOME"' EXIT

if HOME="$TESTHOME" sh install.sh -f >/dev/null 2>&1; then
    # Verify each skill was installed
    for skill in skills/*/; do
        name="$(basename "$skill")"
        target="$TESTHOME/.claude/skills/$name/SKILL.md"
        if [ -f "$target" ]; then
            pass "install: $name → $target"
        else
            fail "install: $name not at $target"
        fi
    done
else
    fail "install.sh: failed to run with HOME=$TESTHOME"
fi

# ----------------------------------------------------------------------
echo ""
echo "[4] README has required sections"

for section in "## Install" "## Usage" "## State files" "## License"; do
    if grep -qF "$section" README.md; then
        pass "README: has '$section' section"
    else
        fail "README: missing '$section' section"
    fi
done

# ----------------------------------------------------------------------
echo ""
echo "[5] state file path consistency"
# Each SKILL.md must reference the standard state file path
for f in skills/pause/SKILL.md skills/resume/SKILL.md skills/loops/SKILL.md; do
    name="$(basename "$(dirname "$f")")"
    if grep -qF ".paused-loops.json" "$f"; then
        pass "$name: references .paused-loops.json"
    else
        fail "$name: does not mention .paused-loops.json"
    fi
done

# ----------------------------------------------------------------------
echo ""
echo "summary: $PASS pass, $FAIL fail"
[ "$FAIL" -eq 0 ]
