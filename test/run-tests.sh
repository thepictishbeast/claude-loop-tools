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
echo "[6] feature documentation tests"

# pause must document the history file
if grep -qF "loop-history.jsonl" skills/pause/SKILL.md; then
    pass "pause: documents loop-history.jsonl"
else
    fail "pause: missing loop-history.jsonl reference"
fi

# pause must document the canary auto-add
if grep -qiE "canary|self.check" skills/pause/SKILL.md; then
    pass "pause: documents canary auto-add"
else
    fail "pause: canary auto-add feature undocumented"
fi

# pause must refuse on truncated prompts (safety feature)
if grep -qiE "truncated|refuse" skills/pause/SKILL.md; then
    pass "pause: documents refuse-on-truncated-prompt safety"
else
    fail "pause: missing truncated-prompt safeguard documentation"
fi

# resume must document the interval override feature
if grep -qiE "resume 5m|interval.*override|/resume.*arg" skills/resume/SKILL.md; then
    pass "resume: documents interval override"
else
    fail "resume: interval override feature undocumented"
fi

# resume must document the cron-conversion table
if grep -qE "^\| .*[Pp]attern.*\|.*[Cc]ron" skills/resume/SKILL.md; then
    pass "resume: documents interval→cron conversion table"
else
    fail "resume: missing interval→cron conversion table"
fi

# resume must document the immediate-execute-now behavior (per /loop)
if grep -qiE "execute.*now|don't wait.*first.*fire|first.*iter.*run.*now" skills/resume/SKILL.md; then
    pass "resume: documents execute-now semantics"
else
    fail "resume: execute-now semantics undocumented"
fi

# loops must document the three views (active + paused + history)
if grep -qiE "active" skills/loops/SKILL.md && \
   grep -qiE "paused" skills/loops/SKILL.md && \
   grep -qiE "history" skills/loops/SKILL.md; then
    pass "loops: documents all three views"
else
    fail "loops: missing one of {active, paused, history} view"
fi

# ----------------------------------------------------------------------
echo ""
echo "[7] example files validate"

if [ -d examples ]; then
    # paused-loops.example.json must be valid JSON and an array
    if [ -f examples/paused-loops.example.json ]; then
        if python3 -c "
import json, sys
d = json.load(open('examples/paused-loops.example.json'))
assert isinstance(d, list), 'top level must be array'
for entry in d:
    for field in ('cron','prompt','recurring'):
        assert field in entry, f'entry missing {field}'
" 2>/dev/null; then
            pass "examples/paused-loops.example.json: valid JSON with required fields"
        else
            fail "examples/paused-loops.example.json: invalid or missing required fields"
        fi
    else
        fail "examples/paused-loops.example.json: file missing"
    fi

    # loop-history.example.jsonl must be one valid JSON object per line
    if [ -f examples/loop-history.example.jsonl ]; then
        if python3 -c "
import json
with open('examples/loop-history.example.jsonl') as f:
    for i, line in enumerate(f, 1):
        line = line.strip()
        if not line:
            continue
        d = json.loads(line)
        assert 'event' in d, f'line {i} missing event field'
        assert 'at' in d, f'line {i} missing at field'
" 2>/dev/null; then
            pass "examples/loop-history.example.jsonl: valid JSONL with required fields"
        else
            fail "examples/loop-history.example.jsonl: invalid or missing required fields"
        fi
    else
        fail "examples/loop-history.example.jsonl: file missing"
    fi

    # examples/README.md must explain both files
    if [ -f examples/README.md ]; then
        if grep -qF "paused-loops.example.json" examples/README.md && \
           grep -qF "loop-history.example.jsonl" examples/README.md; then
            pass "examples/README.md: documents both example files"
        else
            fail "examples/README.md: doesn't reference both example files"
        fi
    else
        fail "examples/README.md: file missing"
    fi
else
    fail "examples/ directory missing"
fi

# ----------------------------------------------------------------------
echo ""
echo "[8] CLAUDE.md (loop hygiene doctrine)"

if [ -f CLAUDE.md ]; then
    pass "CLAUDE.md: file exists"
else
    fail "CLAUDE.md: missing (loop hygiene doctrine for AI agents)"
fi

# Must document the "don't restart on every fire" rule
if grep -qiE "fire is a SIGNAL to continue|don't.*restart|finish.*current" CLAUDE.md 2>/dev/null; then
    pass "CLAUDE.md: documents don't-restart-on-fire rule"
else
    fail "CLAUDE.md: missing the don't-restart-on-fire rule"
fi

# Must mention task list integration
if grep -qiE "TaskList|TaskCreate|task list" CLAUDE.md 2>/dev/null; then
    pass "CLAUDE.md: references task-list integration"
else
    fail "CLAUDE.md: missing task-list integration guidance"
fi

# Must explain tight-tick vs substantive
if grep -qiE "tight.tick|steady.state|substantive" CLAUDE.md 2>/dev/null; then
    pass "CLAUDE.md: distinguishes tight-tick vs substantive iters"
else
    fail "CLAUDE.md: missing tight-tick/substantive distinction"
fi

# README must point at CLAUDE.md
if grep -qF "CLAUDE.md" README.md; then
    pass "README: references CLAUDE.md"
else
    fail "README: missing CLAUDE.md reference"
fi

# loops SKILL.md must document the inflight-task view
if grep -qiE "Inflight tasks|TaskList|task state" skills/loops/SKILL.md; then
    pass "loops skill: documents inflight-task view"
else
    fail "loops skill: missing inflight-task view"
fi

# ----------------------------------------------------------------------
echo ""
echo "summary: $PASS pass, $FAIL fail"
[ "$FAIL" -eq 0 ]
