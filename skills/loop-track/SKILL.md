---
name: loop-track
description: Register an existing cron job into the loop history when it wasn't created via this toolkit. Use when the user has loops they created via raw /loop (not through the loop-tools skills) and wants them visible in /loops history, or when an inherited session has untracked active crons. Logs a 'discovered' event so future /loops and /loop-pause runs see prior context.
---

# /loop-track — register an external cron into the history

## When to use this

- A loop was created via raw `/loop <interval> <prompt>` and no
  history entry exists for it (i.e. it predates this toolkit OR was
  spawned outside the loop-* skills).
- The user wants to attach a `label` to an existing loop so it shows
  up nicely in `/loops` output.
- Auditing: the user wants every active loop to have a `created` or
  `discovered` history event so the audit trail is complete.

`/loops` does an automatic untracked-cron detection pass and logs
`discovered` events as a side effect. `/loop-track` is the explicit
manual version when the user wants to add a label or specific
metadata.

## Argument parsing

```
/loop-track <job-id> [label: "<label>"] [created-at: "<ISO-8601>"]
```

- `<job-id>` — the cron job ID from `CronList`. Required.
- `label:` — optional short label (informational only)
- `created-at:` — optional historical timestamp (default: now,
  marked as `discovered` event)

If no args: list current `CronList` IDs and ask which to track.

## Steps

1. `CronList` — verify the job-id exists. If not: error out.

2. Pull the cron's cron expression + prompt from CronList output.
   **Note**: CronList truncates the prompt to ~80 chars. If
   truncated, refuse to register without the full prompt OR ask the
   user to paste it.

3. Read `~/.claude/loop-history.jsonl`. If a `created`, `resumed`,
   or `discovered` event with this `id_original`, `id_new`, or
   matching `cron + prompt` already exists, this loop is already
   tracked. Tell the user, don't double-log.

4. Append history event:
   ```jsonl
   {"event":"discovered","at":"<ISO-8601>","id":"<job-id>","cron":"<expr>","prompt":"<full or truncated>","label":"<label or null>","reason":"explicit /loop-track"}
   ```

   If `created-at` was passed and is in the past, the event reads
   `{"event":"created","at":"<created-at>","retroactive":true,...}`.

5. `chmod 600 ~/.claude/loop-history.jsonl` if it didn't exist
   before.

6. Confirm — one short line: "tracked job X with label Y."

## Don't

- Don't modify the cron itself (that's `/loop-edit`'s job)
- Don't add to `.paused-loops.json` — this loop is ACTIVE, not paused
- Don't double-log if a history entry already exists

## See also

- `/loops` — does auto-discovery as a side effect of listing
- `/loop-pause`, `/loop-edit`, `/loop-stop` — all check history and
  warn/error if a cron has no track record
