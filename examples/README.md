# Examples

Sample state files showing the shapes the skills read/write.

## `paused-loops.example.json`

The on-disk format at `~/.claude/.paused-loops.json`. Array of paused
loop entries. Each entry has:

| Field | Type | Purpose |
|-------|------|---------|
| `id_original` | string | Original cron job ID before pause (informational) |
| `cron` | string | Cron expression (5-field), e.g. `*/5 * * * *` |
| `cadence_human` | string | Human-readable cadence (informational) |
| `recurring` | bool | Whether the cron auto-renews |
| `prompt` | string | The verbatim prompt Claude receives each fire |
| `canary_added` | bool | Whether `/loop-pause` auto-added a canary line |
| `paused_at` | string | ISO-8601 UTC timestamp |
| `label` | string | Optional short label (informational) |

To resume, `/loop-resume` reads this file and `CronCreate`s each entry. To
change the prompt or any other field before resume, hand-edit this file
between `/loop-pause` and `/loop-resume`.

## `loop-history.example.jsonl`

The append-only audit log at `~/.claude/loop-history.jsonl`. One JSON
object per line. Events:

- `started` — when a loop was first scheduled (`/loop` or `/loop-resume`)
- `paused` — when `/loop-pause` saved + cancelled it
- `resumed` — when `/loop-resume` recreated the cron
- `stopped` — when the loop was permanently cancelled (no resume)
- `iter` — optional, for manual milestone logging within a long-running loop

Common fields: `event`, `at` (ISO UTC), `id_original`, `cron`, `label`.
Resume events add `id_new` and `interval_override`.

## Why JSON / JSONL?

- Plain text — paul can `cat`, `jq`, or `$EDITOR` them anytime
- JSONL for history because it's append-only and crash-safe (no
  need to rewrite the whole file on each event)
- JSON for paused-loops because the skills always read+write the
  whole file atomically

## Mode 600

Both files contain prompt text which may carry sensitive context.
The skills `chmod 600` after writing. The examples ship as 644 for
reading but real instances should be 600.
