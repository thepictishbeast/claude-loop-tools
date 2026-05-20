---
name: file-adr
description: Scaffold a new Architecture Decision Record (Nygard format) + update the ADR index. Picks the next NNN number, writes the file with frontmatter + standard section skeleton, adds the row to docs/adr/README.md. Use when a non-trivial design decision needs explicit sign-off.
---

# /file-adr — scaffold an ADR + update the index

Thin wrapper around the `file-adr` binary (installed at
`~/.local/bin/file-adr`; built by `install.sh`). Binary owns:
next-number scan, file write, index-table update. Agent fills in
the body.

## Steps

1. **Invoke the binary**:

   ```sh
   file-adr --slug <kebab-case-slug> \
            --title "<human-readable title>" \
            [--tracker <task-id-or-issue>] \
            [--status Proposed] \
            [--adr-dir docs/adr]
   ```

   Stdout is the path of the new ADR file (e.g. `docs/adr/019-foo.md`).
   The binary also appends a row to `docs/adr/README.md`.

2. **Open the new ADR file and fill in the body sections**:
   Context · Decision framework · Proposal · Alternatives rejected ·
   Acceptance criteria · Trade-offs accepted · Open questions for
   USER · Cross-references.

3. **If the ADR closes a tracker task**, mark that task in_progress
   via TaskUpdate (or completed if the ADR itself IS the deliverable).

4. **Commit + push** with message
   `[ADR-NNN + #<tracker>] propose <decision>`.

## Net visible tool calls per ADR

**3–5** total: `Bash` (file-adr) + `Read` (open file) + `Edit`/`Write`
(fill body) + optional `TaskUpdate` + `Bash` (commit/push).

Down from ~8 in the prior manual flow (separate Write for ADR,
Edit for README index, TaskUpdate, commit, push — plus mental
overhead of picking the next number).

## Constraints

- `--slug` MUST be kebab-case lowercase ASCII (binary enforces).
- `--adr-dir` defaults to `docs/adr` (per the LFI convention).
- Status defaults to `Proposed` — change to `Accepted` only
  after USER sign-off, in a separate commit.
