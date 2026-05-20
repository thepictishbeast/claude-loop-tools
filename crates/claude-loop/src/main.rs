//! `claude-loop` — atomic CLI for the loop-pause / loop-resume /
//! loops / loop-history skills.
//!
//! Token-efficient Rust replacement for the prior markdown-skill
//! orchestration. Each subcommand collapses what was previously
//! 3-5 separate agent-visible Bash + Write + Read tool calls into
//! ONE Bash invocation. The agent still has to call the Cron* tool
//! API directly (CronList / CronCreate / CronDelete) since those
//! have no shell surface — but every state file + chmod + history
//! op is owned by this binary now.
//!
//! Per AVP-2: explicit error handling via anyhow, tested arg
//! parsing via clap derives, no shell-quoting footguns.

use anyhow::{Context, Result};
use chrono::Utc;
use clap::{Parser, Subcommand};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "claude-loop",
    about = "Atomic CLI for /loop-pause /loop-resume /loops",
    version
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,

    /// Override the state directory (default $HOME/.claude/).
    #[arg(long, global = true)]
    state_dir: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Read CronList JSON on stdin, write paused-loops.json, log
    /// pause event, output the IDs to CronDelete.
    Pause {
        /// Label the pause set (free-form; default derived from
        /// the first job's prompt).
        #[arg(long)]
        label: Option<String>,
    },

    /// Read paused-loops.json, output the jobs to CronCreate as
    /// JSON lines (one per job), log resume event, delete the
    /// state file. Optional interval override applies to ALL
    /// resumed jobs.
    Resume {
        /// Interval override: `5m`, `2h`, `1d`, etc. Converts to
        /// cron internally. Default: use saved cron expr.
        #[arg(long)]
        interval: Option<String>,
    },

    /// Print active + paused + recent-history summary (read-only).
    List,

    /// Print loop-history.jsonl tail (read-only).
    History {
        /// Number of recent lines (default 20).
        #[arg(short = 'n', long, default_value_t = 20)]
        lines: usize,

        /// Restrict to lines whose `event` field equals this value
        /// (e.g. `--filter event=stopped`). Repeatable: each --filter
        /// AND-narrows the result. Lines that fail to parse as JSON
        /// are skipped silently when any filter is provided.
        #[arg(long, value_parser = parse_filter)]
        filter: Vec<(String, String)>,
    },
}

/// Parse `--filter key=value` into `(key, value)`. Refused `key=`,
/// refused `=value`, refused bare `key`. Value may contain `=`
/// (only the first `=` is the separator).
fn parse_filter(s: &str) -> Result<(String, String), String> {
    let Some((k, v)) = s.split_once('=') else {
        return Err(format!("filter expected key=value, got `{s}`"));
    };
    if k.is_empty() {
        return Err(format!("filter key empty in `{s}`"));
    }
    if v.is_empty() {
        return Err(format!("filter value empty in `{s}`"));
    }
    Ok((k.to_string(), v.to_string()))
}

#[derive(Serialize, Deserialize, Debug)]
struct PausedJob {
    id_original: String,
    cron: String,
    cadence_human: String,
    recurring: bool,
    prompt: String,
    canary_added: bool,
    paused_at: String,
    label: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    inflight_tasks: Vec<InflightTask>,
}

#[derive(Serialize, Deserialize, Debug)]
struct InflightTask {
    id: String,
    status: String,
    subject: String,
    #[serde(rename = "activeForm", default)]
    active_form: Option<String>,
}

/// CronList JSON input shape (passed on stdin to `pause`).
/// Caller (the agent) constructs this from the CronList tool
/// output. Minimal fields — everything else `pause` derives.
#[derive(Deserialize, Debug)]
struct CronListEntry {
    id: String,
    cron: String,
    #[serde(default = "default_recurring")]
    recurring: bool,
    prompt: String,
    #[serde(default)]
    cadence_human: Option<String>,
    #[serde(default)]
    inflight_tasks: Vec<InflightTask>,
}

fn default_recurring() -> bool {
    true
}

fn state_dir(override_: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(p) = override_ {
        return Ok(p);
    }
    let home = std::env::var("HOME").context("HOME not set")?;
    Ok(PathBuf::from(home).join(".claude"))
}

fn paused_path(dir: &std::path::Path) -> PathBuf {
    dir.join(".paused-loops.json")
}

fn history_path(dir: &std::path::Path) -> PathBuf {
    dir.join("loop-history.jsonl")
}

fn paused_lock(dir: &std::path::Path) -> PathBuf {
    dir.join(".paused-loops.lock")
}

fn history_lock(dir: &std::path::Path) -> PathBuf {
    dir.join("loop-history.lock")
}

/// AVP-2: 600 perms on state + history (prompts may carry
/// sensitive context).
fn chmod_600(p: &std::path::Path) -> Result<()> {
    let mut perms = fs::metadata(p)?.permissions();
    perms.set_mode(0o600);
    fs::set_permissions(p, perms)?;
    Ok(())
}

/// Append a single JSON event line to history, under flock.
fn history_append(dir: &std::path::Path, event: &serde_json::Value) -> Result<()> {
    let lock = File::create(history_lock(dir))?;
    lock.lock_exclusive()?;
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(history_path(dir))?;
    writeln!(f, "{}", serde_json::to_string(event)?)?;
    chmod_600(&history_path(dir))?;
    Ok(())
}

fn cmd_pause(dir: &std::path::Path, label_override: Option<String>) -> Result<()> {
    let mut stdin_buf = String::new();
    std::io::Read::read_to_string(&mut std::io::stdin(), &mut stdin_buf)?;
    let entries: Vec<CronListEntry> = serde_json::from_str(&stdin_buf)
        .context("stdin must be a JSON array of CronListEntry — see --help")?;

    if entries.is_empty() {
        // Print empty JSON list so agent doesn't try to CronDelete
        // anything. Matches the skill spec's "no active jobs" path.
        println!("[]");
        return Ok(());
    }

    let now = Utc::now().to_rfc3339();
    let jobs: Vec<PausedJob> = entries
        .into_iter()
        .map(|e| {
            let canary_added = !has_canary(&e.prompt);
            let prompt = if canary_added {
                format!(
                    "{}\n\nNote: if you canceled or stopped this loop, you should NOT be seeing this message.",
                    e.prompt
                )
            } else {
                e.prompt
            };
            let label = label_override
                .clone()
                .unwrap_or_else(|| infer_label(&prompt));
            PausedJob {
                id_original: e.id,
                cron: e.cron,
                cadence_human: e.cadence_human.unwrap_or_else(|| "(unspecified)".into()),
                recurring: e.recurring,
                prompt,
                canary_added,
                paused_at: now.clone(),
                label,
                inflight_tasks: e.inflight_tasks,
            }
        })
        .collect();

    // Merge with prior state if present.
    let pp = paused_path(dir);
    let mut all: Vec<PausedJob> = if pp.exists() {
        let prior_raw = fs::read_to_string(&pp)?;
        serde_json::from_str(&prior_raw).unwrap_or_default()
    } else {
        Vec::new()
    };
    all.extend(jobs);

    // Write under flock.
    let lock = File::create(paused_lock(dir))?;
    lock.lock_exclusive()?;
    fs::write(&pp, serde_json::to_string_pretty(&all)?)?;
    chmod_600(&pp)?;

    // Append one history event per paused job.
    for j in &all {
        history_append(
            dir,
            &serde_json::json!({
                "event": "paused",
                "at": &now,
                "id_original": &j.id_original,
                "cron": &j.cron,
                "label": &j.label,
            }),
        )?;
    }

    // Output the IDs the agent should CronDelete.
    let ids: Vec<&str> = all.iter().map(|j| j.id_original.as_str()).collect();
    println!("{}", serde_json::to_string(&ids)?);
    Ok(())
}

fn cmd_resume(dir: &std::path::Path, interval_override: Option<String>) -> Result<()> {
    let pp = paused_path(dir);
    if !pp.exists() {
        eprintln!("no paused loops to resume");
        println!("[]");
        return Ok(());
    }
    let raw = fs::read_to_string(&pp)?;
    let jobs: Vec<PausedJob> =
        serde_json::from_str(&raw).context("paused-loops.json malformed")?;
    if jobs.is_empty() {
        println!("[]");
        return Ok(());
    }

    let cron_override = interval_override
        .as_deref()
        .map(interval_to_cron)
        .transpose()?;

    // Output JSON the agent uses to drive CronCreate calls.
    let out: Vec<_> = jobs
        .iter()
        .map(|j| {
            serde_json::json!({
                "cron": cron_override.clone().unwrap_or_else(|| j.cron.clone()),
                "prompt": j.prompt,
                "recurring": j.recurring,
                "label": j.label,
                "id_original": j.id_original,
                "inflight_tasks": j.inflight_tasks.iter().map(|t| {
                    serde_json::json!({
                        "id": t.id,
                        "status": t.status,
                        "subject": t.subject,
                        "activeForm": t.active_form,
                    })
                }).collect::<Vec<_>>(),
            })
        })
        .collect();

    let now = Utc::now().to_rfc3339();
    for j in &jobs {
        history_append(
            dir,
            &serde_json::json!({
                "event": "resumed",
                "at": &now,
                "id_original": &j.id_original,
                "cron": cron_override.clone().unwrap_or_else(|| j.cron.clone()),
                "interval_override": interval_override.clone(),
                "inflight_tasks_replayed": j.inflight_tasks.len(),
            }),
        )?;
    }

    // Consume state file under flock.
    let lock = File::create(paused_lock(dir))?;
    lock.lock_exclusive()?;
    fs::remove_file(&pp)?;

    println!("{}", serde_json::to_string(&out)?);
    Ok(())
}

fn cmd_list(dir: &std::path::Path) -> Result<()> {
    let pp = paused_path(dir);
    let paused: Vec<PausedJob> = if pp.exists() {
        serde_json::from_str(&fs::read_to_string(&pp)?).unwrap_or_default()
    } else {
        Vec::new()
    };
    let hp = history_path(dir);
    let history_tail: Vec<String> = if hp.exists() {
        let raw = fs::read_to_string(&hp)?;
        raw.lines()
            .rev()
            .take(20)
            .map(String::from)
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "paused": paused,
            "history_tail": history_tail,
        }))?
    );
    Ok(())
}

fn cmd_history(
    dir: &std::path::Path,
    lines: usize,
    filters: &[(String, String)],
) -> Result<()> {
    let hp = history_path(dir);
    if !hp.exists() {
        println!("[]");
        return Ok(());
    }
    let raw = fs::read_to_string(&hp)?;
    let filtered: Vec<&str> = if filters.is_empty() {
        raw.lines().rev().take(lines).collect()
    } else {
        // When filters are present we must read the whole file (the
        // last N matching lines are not necessarily in the last N raw
        // lines). The history file caps via the maintainer's external
        // rotation, so this is bounded in practice — still O(file) so
        // worth flagging if it grows large.
        raw.lines()
            .rev()
            .filter(|l| line_matches_filters(l, filters))
            .take(lines)
            .collect()
    };
    let mut tail = filtered;
    tail.reverse();
    for line in tail {
        println!("{}", line);
    }
    Ok(())
}

/// True iff the JSON object on `line` has every (key, value) in
/// `filters` matching (string-equal on the JSON-encoded value's
/// string form). Lines that don't parse as JSON object are skipped
/// (returns false). Empty filter list means "everything matches"
/// but the caller short-circuits this case for perf.
fn line_matches_filters(line: &str, filters: &[(String, String)]) -> bool {
    let Ok(serde_json::Value::Object(map)) = serde_json::from_str::<serde_json::Value>(line)
    else {
        return false;
    };
    filters.iter().all(|(k, v)| {
        let actual = map.get(k);
        match actual {
            Some(serde_json::Value::String(s)) => s == v,
            // Numeric / boolean / nested fields don't have a borrow-able
            // string view so we serialize them to compare. The cmp_owned
            // lint flags this allocation but the filter pathway is bounded
            // by file size and the data shapes we accept here are all
            // small primitives — readability wins.
            #[allow(clippy::cmp_owned)]
            Some(serde_json::Value::Number(n)) => n.to_string() == *v,
            #[allow(clippy::cmp_owned)]
            Some(serde_json::Value::Bool(b)) => b.to_string() == *v,
            #[allow(clippy::cmp_owned)]
            Some(other) => other.to_string() == *v,
            None => false,
        }
    })
}

/// Detect the auto-canary line that /loop-pause adds. Same regexes
/// the skill spec checks for.
fn has_canary(prompt: &str) -> bool {
    let lower = prompt.to_lowercase();
    lower.contains("cancel") && lower.contains("should not") && lower.contains("see")
        || lower.contains("stop") && lower.contains("should not") && lower.contains("see")
}

/// Infer a short label from the first sentence of the prompt.
fn infer_label(prompt: &str) -> String {
    prompt
        .split(&['.', '\n'][..])
        .next()
        .unwrap_or(prompt)
        .chars()
        .take(60)
        .collect()
}

/// Interval `Nm` / `Nh` / `Nd` → cron expression. Matches the
/// /loop and /loop-resume skill's conversion table.
fn interval_to_cron(s: &str) -> Result<String> {
    let (n_str, unit) = s.split_at(s.len().saturating_sub(1));
    let n: u64 = n_str
        .parse()
        .with_context(|| format!("interval prefix not numeric: {s}"))?;
    let cron = match unit {
        "s" => {
            // Round up to 1m.
            "*/1 * * * *".to_string()
        }
        "m" if n <= 59 => format!("*/{} * * * *", n),
        "m" => {
            let h = n / 60;
            if 24 % h != 0 {
                anyhow::bail!("interval {s} doesn't cleanly divide hours");
            }
            format!("0 */{} * * *", h)
        }
        "h" if n <= 23 => format!("0 */{} * * *", n),
        "d" => format!("0 0 */{} * *", n),
        _ => anyhow::bail!("unsupported interval unit in {s}"),
    };
    Ok(cron)
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let dir = state_dir(cli.state_dir.clone())?;
    fs::create_dir_all(&dir)?;
    match cli.cmd {
        Cmd::Pause { label } => cmd_pause(&dir, label),
        Cmd::Resume { interval } => cmd_resume(&dir, interval),
        Cmd::List => cmd_list(&dir),
        Cmd::History { lines, filter } => cmd_history(&dir, lines, &filter),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interval_5m() {
        assert_eq!(interval_to_cron("5m").unwrap(), "*/5 * * * *");
    }
    #[test]
    fn interval_2h() {
        assert_eq!(interval_to_cron("2h").unwrap(), "0 */2 * * *");
    }
    #[test]
    fn interval_1d() {
        assert_eq!(interval_to_cron("1d").unwrap(), "0 0 */1 * *");
    }
    #[test]
    fn interval_60m_rounds_to_1h() {
        assert_eq!(interval_to_cron("60m").unwrap(), "0 */1 * * *");
    }
    #[test]
    fn interval_120m_to_2h() {
        assert_eq!(interval_to_cron("120m").unwrap(), "0 */2 * * *");
    }
    #[test]
    fn canary_detects_standard_line() {
        assert!(has_canary(
            "do stuff. if you canceled or stopped this loop, you should NOT be seeing this message."
        ));
    }
    #[test]
    fn canary_absent_on_bare_prompt() {
        assert!(!has_canary("do stuff every 5 minutes"));
    }
    #[test]
    fn infer_label_truncates_to_60() {
        let s = "a".repeat(100);
        assert_eq!(infer_label(&s).len(), 60);
    }
    #[test]
    fn infer_label_first_sentence() {
        assert_eq!(infer_label("First. Second."), "First");
    }

    #[test]
    fn parse_filter_rejects_missing_eq() {
        assert!(parse_filter("event").is_err());
    }
    #[test]
    fn parse_filter_rejects_empty_key() {
        assert!(parse_filter("=stopped").is_err());
    }
    #[test]
    fn parse_filter_rejects_empty_value() {
        assert!(parse_filter("event=").is_err());
    }
    #[test]
    fn parse_filter_accepts_value_with_equals() {
        // Only the FIRST = separates; everything after is value.
        assert_eq!(
            parse_filter("k=a=b").unwrap(),
            ("k".to_string(), "a=b".to_string())
        );
    }
    #[test]
    fn parse_filter_accepts_event_stopped() {
        assert_eq!(
            parse_filter("event=stopped").unwrap(),
            ("event".to_string(), "stopped".to_string())
        );
    }

    #[test]
    fn filter_matches_string_event() {
        let line = r#"{"event":"stopped","id":"abc"}"#;
        let f = vec![("event".to_string(), "stopped".to_string())];
        assert!(line_matches_filters(line, &f));
    }
    #[test]
    fn filter_rejects_different_value() {
        let line = r#"{"event":"started","id":"abc"}"#;
        let f = vec![("event".to_string(), "stopped".to_string())];
        assert!(!line_matches_filters(line, &f));
    }
    #[test]
    fn filter_rejects_missing_key() {
        let line = r#"{"id":"abc"}"#;
        let f = vec![("event".to_string(), "stopped".to_string())];
        assert!(!line_matches_filters(line, &f));
    }
    #[test]
    fn filter_skips_nonjson_line() {
        // Non-JSON history line should NOT match any filter — fail
        // closed rather than panicking.
        let f = vec![("event".to_string(), "x".to_string())];
        assert!(!line_matches_filters("not-json", &f));
    }
    #[test]
    fn filter_ands_multiple_keys() {
        let line = r#"{"event":"stopped","id":"abc"}"#;
        let f = vec![
            ("event".to_string(), "stopped".to_string()),
            ("id".to_string(), "abc".to_string()),
        ];
        assert!(line_matches_filters(line, &f));
        // One mismatch fails the AND.
        let f2 = vec![
            ("event".to_string(), "stopped".to_string()),
            ("id".to_string(), "other".to_string()),
        ];
        assert!(!line_matches_filters(line, &f2));
    }
    #[test]
    fn filter_matches_numeric_field() {
        // Numbers coerce to string-form for comparison.
        let line = r#"{"event":"fired","duration_ms":150}"#;
        let f = vec![("duration_ms".to_string(), "150".to_string())];
        assert!(line_matches_filters(line, &f));
    }
    #[test]
    fn filter_matches_bool_field() {
        let line = r#"{"event":"paused","recurring":true}"#;
        let f = vec![("recurring".to_string(), "true".to_string())];
        assert!(line_matches_filters(line, &f));
    }
}
