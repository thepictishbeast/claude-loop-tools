---
name: regression-guard
description: Generate a Rust #[test] skeleton that anchors a refactor commit. Reads the commit's diff, auto-detects the refactor pattern (let-else / let-Ok-else / warn-continue / SAFETY-annot / graceful-degrade / struct-update / error-return), emits a test fn skeleton with REGRESSION-GUARD docstring back-ref so reverting the fix surfaces the commit in test output.
---

# /regression-guard — emit a REGRESSION-GUARD test stub for a refactor commit

Thin wrapper around the `regression-guard` binary (installed at
`~/.local/bin/regression-guard`; built by `install.sh`). Binary
owns: `git show` of the commit, diff pattern auto-detection, test
skeleton rendering. Agent fills in the test body.

## Steps

1. **Invoke the binary** with the refactor commit hash:

   ```sh
   regression-guard <commit-hash> \
                    [--pattern <override>] \
                    [--test-name <fn-name>] \
                    [--module-path <crate::path::to::mod>] \
                    [--git-dir <repo-path>]
   ```

   Stdout is a Rust source snippet (test fn + REGRESSION-GUARD
   docstring referencing the commit + the detected pattern).

2. **Paste the snippet into the test module** of the file the
   refactor touched (typically `src/<file>.rs`'s
   `#[cfg(test)] mod tests` block).

3. **Fill in the test body** with the actual exercise of the
   pathological input the refactor defends against. Examples per
   pattern are listed in the snippet's TODO comment.

4. **Run the test**: `cargo test <test_name>` — it MUST pass on the
   post-refactor code (that's the point — it locks in the new
   behaviour). If it fails, the test isn't asserting the right
   thing; revisit.

5. **Commit** the test in the same PR / commit-chain as the
   refactor, or in a `[REGRESSION-GUARD]`-prefixed follow-up
   commit referencing the refactor hash.

## Net visible tool calls per regression-guard

**2–4** total: `Bash` (regression-guard) + `Edit` (paste snippet) +
`Bash` (cargo test) + optional `Bash` (commit).

Down from ~6 in the prior manual flow (read commit, identify
pattern in head, write test from scratch, write docstring, copy
hash, etc.).

## Detected patterns

| Pattern | Trigger in diff |
|---|---|
| `let-else` | `.unwrap()` removed + `let Some(x) = ... else` added |
| `let-ok-else` | `.unwrap_or_else(\|_\| panic!())` removed + `warn!() + let Ok` added |
| `warn-continue` | `if let Ok(x) = stream` removed + `match + warn + continue` or `.flatten()` added |
| `safety-annot` | `// SAFETY:` annotation added |
| `graceful-degrade` | `.expect()` removed + `ok_or_else / ? / Err / else {` added |
| `struct-update` | `T::default(); x.f = v;` removed + `..Default::default()` added |
| `error-return` | `panic!()` removed + `return Err(...)` added |

Auto-detection covers the common cases from the LFI 2026-05-19
doctrine sweep (31 violations cleared across 9 files). For
patterns not covered, pass `--pattern unknown` and edit the
docstring manually.
