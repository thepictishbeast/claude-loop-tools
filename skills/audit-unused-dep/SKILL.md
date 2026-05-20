---
name: audit-unused-dep
description: Scan a Cargo.toml + src/ tree for zero-call-site direct dependencies. Reports candidates with confidence levels; doesn't modify Cargo.toml. Faster + more inspectable than cargo-machete for the common case; complements machete (not a replacement).
---

# /audit-unused-dep — list direct deps with no code references

Thin wrapper around the `audit-unused-dep` binary (installed at
`~/.local/bin/audit-unused-dep`; built by `install.sh`). Binary
owns: Cargo.toml parse, recursive .rs scan, comment-stripping, +/-
grep, confidence classification.

## Steps

1. **Invoke the binary** (defaults to current dir's Cargo.toml + src/):

   ```sh
   audit-unused-dep [--manifest Cargo.toml] [--src src] [--extra-dir bin] [--human]
   ```

   Stdout is JSON (default) or a human-readable table (`--human`).
   Each candidate carries:
   - `name`
   - `confidence`: `high` (0 matches anywhere) or `medium` (0
     in code, N in comments — verify before removing)
   - `rationale`

2. **Triage each `high`-confidence candidate**:
   - Verify against `cargo tree -i <name>` (sometimes a transitive
     pulls the same crate; removing the direct dep doesn't help)
   - Check Cargo.toml `[features]` — features may reference deps
     without `use` in source
   - Remove the line from `Cargo.toml`, run `cargo build`,
     run any relevant tests. If clean, commit.

3. **Triage `medium`-confidence candidates** more carefully — the
   comment matches mean the crate name appears in a doc-string or
   string literal. Could be:
   - Documentation reference (keep dep)
   - Static-analysis rule that names the crate (keep dep)
   - Stale comment from a removed call site (safe to remove)

## Net visible tool calls per audit

**1–2** total: `Bash` (audit-unused-dep) + optional `Bash`
(cargo tree -i for verification).

Compared with cargo-machete: this tool is more inspectable
(rationale per candidate) but slightly noisier (medium-confidence
candidates are FPs for machete-style "strict no-comments" rules).
Use both for cross-check.

## When to NOT remove

- Feature-flag-only deps (only `use`d under `#[cfg(feature = "X")]`
  that's off by default) — false-positive for both tools.
- Build-time-only deps that show up as `[build-dependencies]`
  (this tool already ignores them).
- proc-macro deps without an explicit `use` (rare but possible
  — e.g. some derives bring their crate into scope implicitly).
