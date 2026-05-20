---
name: workspace-tests
description: Run cargo test --workspace across a list of Rust workspaces, failing fast and reporting which repo broke. Surfaces sibling-workspace regressions that single-repo hygiene cycles miss. Use when the user says "verify cross-repo tests", "run hygiene", "anything broken in sibling workspaces?", or as part of a /loop hygiene priority.
---

# /workspace-tests — cargo test across N Rust workspaces

Walks a configured list of in-scope Rust workspaces and runs
`cargo test --workspace` against each. Fails fast (stops at first
red workspace) by default; `--continue-on-error` runs the full set.
Reports total + per-repo pass/fail counts.

## Why this exists

Surfaced 2026-05-20 from PlausiDen-Forge #303: an opportunistic
hygiene cycle that ran `cargo test --workspace` only on the
currently-active repo silently missed a real E0063 in Loom
(`CmsSection::Form` scaffold missing `style` field). Cross-repo
discovery only happened when the next agent ran `cargo fmt` and
hit the compile error.

Real failure mode: a substrate-wide field addition lands in one
crate but the consumer-side scaffolds in sibling repos don't
update. Tests still pass in the producer crate; consumer crate
breaks at HEAD. Without explicit cross-repo coverage, hours of
"green builds" accumulate before the gap surfaces.

## Steps

### 1. Read the workspace list

Default in-scope set (PlausiDen substrate):

```
/home/paul/projects/PlausiDen-Forge
/home/paul/projects/PlausiDen-Loom
/home/paul/projects/PlausiDen-Crawler
/home/paul/projects/Crucible
```

LFI repos (PlausiDen-LFI, Forge-LFI) are excluded by default —
they have a dedicated Claude instance per memory
`feedback_lfi_out_of_scope_for_this_instance`. Add `--include-lfi`
to override.

User can override the set via:
- `$WORKSPACE_TESTS_REPOS` env (colon-separated absolute paths)
- `~/.config/workspace-tests/repos.txt` (one path per line)

### 2. Per repo, run

```sh
sudo -u paul cargo test --workspace --manifest-path "$REPO/Cargo.toml" 2>&1 \
  | tee "/tmp/workspace-tests-$(basename "$REPO").log" \
  | grep -E '^test result|FAILED|^error'
```

Capture the trailing aggregate counts via:

```sh
sudo -u paul cargo test --workspace --manifest-path "$REPO/Cargo.toml" 2>&1 \
  | grep '^test result' \
  | awk '{s+=$4; f+=$6} END {print s,"passed,",f,"failed"}'
```

Skip repos with no `Cargo.toml` at the root (surface a warning;
don't fail).

### 3. Fail-fast vs continue

- Default: bail at first non-zero exit. Report which repo broke.
- `--continue-on-error`: run all repos, then exit non-zero if any
  failed.

### 4. Output format

```
PlausiDen-Forge       1881 passed, 0 failed
PlausiDen-Loom        1335 passed, 0 failed
PlausiDen-Crawler     1084 passed, 0 failed
Crucible                15 passed, 0 failed
-----
TOTAL                 4315 passed, 0 failed across 4 repos
```

On failure, the broken repo's tail-50 of test output prints
verbatim before the summary, with a `===  <repo>: REGRESSION ===`
banner so the agent / operator can grep for the boundary.

### 5. Bypass conditions

- Empty workspace list → exit 0 with "no repos configured" warning
- `cargo` not installed → exit 2 with the install URL
- Any repo path doesn't exist → warn + skip, don't fail

### 6. Recommended invocation

Inline in /loop hygiene priorities:

```sh
sh ~/.local/share/claude-tools/lib/workspace-tests.sh --continue-on-error
```

Or, when the loop fires and you have ~30s of budget:

```sh
sh ~/.local/share/claude-tools/lib/workspace-tests.sh
# fails fast — first red repo aborts; agent then drives into the fix
```

## Related skills

- `regression-guard` — adjacent: catches per-PR regressions on a single repo
- `loop-health` — checks loop / cron state but not workspace test state
- `audit-unused-dep` — per-repo dep audit; complements workspace-tests as a hygiene pair

## Don't

- Don't recurse into nested Cargo.tomls (only the workspace root).
- Don't run `cargo build` separately — `cargo test --workspace`
  exercises the full compile path.
- Don't auto-fix detected failures. The skill is read-only; the
  agent decides whether to attempt a fix in the surfacing repo.
- Don't include LFI repos by default per the dedicated-instance
  doctrine.
