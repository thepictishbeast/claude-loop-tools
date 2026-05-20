#!/bin/sh
# workspace-tests — cargo test --workspace across N Rust workspaces
#
# Surfaces sibling-workspace regressions that single-repo hygiene
# cycles miss. See SKILL.md for the why; this is the body.
#
# Usage:
#   workspace-tests.sh                       # in-scope default set, fail-fast
#   workspace-tests.sh --continue-on-error   # run all, exit non-zero if any fail
#   workspace-tests.sh --include-lfi         # include the LFI repos
#   workspace-tests.sh /path/A /path/B ...   # override repo set explicitly
#
# Exit codes:
#   0  all green
#   1  one or more repos red (per fail-fast / continue mode)
#   2  cargo not installed
#   3  no repos to run (empty workspace list)

set -eu

CONTINUE_ON_ERROR=0
INCLUDE_LFI=0
REPOS=""

for arg in "$@"; do
    case "$arg" in
        --continue-on-error|-c)
            CONTINUE_ON_ERROR=1 ;;
        --include-lfi)
            INCLUDE_LFI=1 ;;
        --help|-h)
            echo "usage: $0 [--continue-on-error] [--include-lfi] [<repo> ...]"
            exit 0
            ;;
        --*)
            echo >&2 "unknown flag: $arg"
            exit 2
            ;;
        *)
            REPOS="$REPOS $arg" ;;
    esac
done

if ! command -v cargo > /dev/null 2>&1; then
    echo >&2 "error: cargo not found in PATH"
    echo >&2 "install via https://rustup.rs"
    exit 2
fi

# Default in-scope PlausiDen substrate set. LFI excluded by default
# per the feedback_lfi_out_of_scope_for_this_instance memory.
if [ -z "$REPOS" ]; then
    if [ -n "${WORKSPACE_TESTS_REPOS:-}" ]; then
        REPOS="$(echo "$WORKSPACE_TESTS_REPOS" | tr ':' ' ')"
    elif [ -f "$HOME/.config/workspace-tests/repos.txt" ]; then
        REPOS="$(grep -v '^#' "$HOME/.config/workspace-tests/repos.txt" | grep -v '^$')"
    else
        REPOS="/home/paul/projects/PlausiDen-Forge
/home/paul/projects/PlausiDen-Loom
/home/paul/projects/PlausiDen-Crawler
/home/paul/projects/Crucible"
        if [ "$INCLUDE_LFI" -eq 1 ]; then
            REPOS="$REPOS
/home/paul/projects/PlausiDen-LFI
/home/paul/projects/Forge-LFI"
        fi
    fi
fi

TOTAL_PASSED=0
TOTAL_FAILED=0
REPOS_OK=0
REPOS_BROKEN=""

for repo in $REPOS; do
    if [ ! -d "$repo" ]; then
        printf '%-32s\twarn: path missing — skipped\n' "$(basename "$repo")"
        continue
    fi
    if [ ! -f "$repo/Cargo.toml" ]; then
        printf '%-32s\twarn: no Cargo.toml — skipped\n' "$(basename "$repo")"
        continue
    fi

    LOG="/tmp/workspace-tests-$(basename "$repo").log"

    # Run the workspace tests; capture exit code via the result file
    # so `set -e` doesn't bail us out before the per-repo report.
    set +e
    if [ "$(id -u)" -eq 0 ] && id paul > /dev/null 2>&1; then
        # Running as root with paul user available — run as paul so
        # target/ writes land with paul's ownership.
        sudo -u paul cargo test --workspace --manifest-path "$repo/Cargo.toml" > "$LOG" 2>&1
    else
        cargo test --workspace --manifest-path "$repo/Cargo.toml" > "$LOG" 2>&1
    fi
    EXIT=$?
    set -e

    PASSED="$(grep '^test result' "$LOG" | awk '{s+=$4} END {print s+0}')"
    FAILED="$(grep '^test result' "$LOG" | awk '{f+=$6} END {print f+0}')"

    if [ "$EXIT" -ne 0 ] || [ "$FAILED" -gt 0 ]; then
        printf '%-32s\t%d passed, %d failed\tBROKEN\n' "$(basename "$repo")" "$PASSED" "$FAILED"
        echo "=== $(basename "$repo"): REGRESSION ==="
        tail -50 "$LOG"
        echo "=== /$(basename "$repo") ==="
        REPOS_BROKEN="$REPOS_BROKEN $(basename "$repo")"
        TOTAL_FAILED=$((TOTAL_FAILED + FAILED + 1))
        if [ "$CONTINUE_ON_ERROR" -eq 0 ]; then
            echo
            echo "FAIL-FAST: $(basename "$repo") broke; aborting."
            echo "Re-run with --continue-on-error to test all repos."
            exit 1
        fi
    else
        printf '%-32s\t%d passed, %d failed\n' "$(basename "$repo")" "$PASSED" "$FAILED"
        TOTAL_PASSED=$((TOTAL_PASSED + PASSED))
        REPOS_OK=$((REPOS_OK + 1))
    fi
done

echo "-----"
if [ -n "$REPOS_BROKEN" ]; then
    printf 'TOTAL\t%d passed, %d failed across %d repos\n' "$TOTAL_PASSED" "$TOTAL_FAILED" "$REPOS_OK"
    echo "BROKEN:$REPOS_BROKEN"
    exit 1
else
    printf 'TOTAL\t%d passed, 0 failed across %d repos\n' "$TOTAL_PASSED" "$REPOS_OK"
fi
