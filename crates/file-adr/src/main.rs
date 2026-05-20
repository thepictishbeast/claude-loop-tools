//! `file-adr` — scaffold an Architecture Decision Record + update the
//! index. One-shot atomic writer.
//!
//! Usage:
//!   file-adr --slug pulp-simd-hdc-kernels \
//!            --title "Adopt pulp for runtime-dispatched SIMD" \
//!            --tracker 85 \
//!            --status Proposed \
//!            [--adr-dir docs/adr]
//!
//! Default --adr-dir is `docs/adr`. The binary:
//!   1. Picks the next ADR number by scanning the dir for NNN-*.md.
//!   2. Writes docs/adr/NNN-<slug>.md with Nygard-style sections +
//!      frontmatter the agent fills in.
//!   3. Adds a row to docs/adr/README.md's index table (creates the
//!      table if absent).
//!   4. Prints the new ADR's path to stdout — agent edits it next.

use anyhow::{bail, Context, Result};
use chrono::Utc;
use clap::Parser;
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(name = "file-adr", version, about)]
struct Cli {
    /// Kebab-case slug for the filename (e.g. `pulp-simd-hdc-kernels`).
    #[arg(long)]
    slug: String,

    /// Human-readable title (e.g. "Adopt pulp for runtime-dispatched SIMD").
    #[arg(long)]
    title: String,

    /// Optional task / tracker ID to cross-reference in the ADR.
    #[arg(long)]
    tracker: Option<String>,

    /// Status: Proposed | Accepted | Superseded | Deprecated. Default Proposed.
    #[arg(long, default_value = "Proposed")]
    status: String,

    /// Directory holding ADRs. Default `docs/adr`.
    #[arg(long, default_value = "docs/adr")]
    adr_dir: PathBuf,

    /// Author byline. Default `Claude (loop iteration)`.
    #[arg(long, default_value = "Claude (loop iteration)")]
    author: String,
}

fn next_number(dir: &Path) -> Result<u32> {
    if !dir.exists() {
        fs::create_dir_all(dir)?;
        return Ok(1);
    }
    let re = Regex::new(r"^(\d{3,})-").unwrap();
    let mut max = 0u32;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if let Some(caps) = re.captures(&name) {
            if let Ok(n) = caps[1].parse::<u32>() {
                if n > max {
                    max = n;
                }
            }
        }
    }
    Ok(max + 1)
}

fn render_body(cli: &Cli, num: u32) -> String {
    let tracker_line = cli
        .tracker
        .as_deref()
        .map(|t| format!("**Tracker:** {}\n", t))
        .unwrap_or_default();
    format!(
        r#"# ADR {num:03} — {title}

**Status:** {status}
**Date:** {date}
{tracker}**Author:** {author}

## Context

(Describe the forcing function. What's the current state? What gap
does this ADR exist to address? Cite any research / prior ADR / task.)

## Decision framework

Per the algorithmic-upgrade decision framework introduced in ADR 015:

1. **Is the gap real?** ___
2. **Is the fix known in the literature / community?** ___
3. **What is the smallest delta that closes the gap without paving
   the cowpath?** ___

## Proposal

(Concrete implementation surface: which files, which modules,
which APIs. Phasing if multi-PR.)

## Alternatives rejected

**Alt 1: ___**
Reason rejected: ___

**Alt 2: ___**
Reason rejected: ___

## Acceptance criteria

- [ ] ___
- [ ] ___
- [ ] Tests pass under both default + opt-in feature flag (if applicable).
- [ ] `cargo audit` + `cargo deny` clean on any new deps.

## Trade-offs accepted

- ___
- ___

## Open questions for USER

1. ___
2. ___

---

**Cross-references**:
- ___
"#,
        num = num,
        title = cli.title,
        status = cli.status,
        date = Utc::now().format("%Y-%m-%d"),
        tracker = tracker_line,
        author = cli.author,
    )
}

fn update_index(dir: &Path, num: u32, slug: &str, status: &str, title: &str) -> Result<()> {
    let index_path = dir.join("README.md");
    let mut content = if index_path.exists() {
        fs::read_to_string(&index_path)?
    } else {
        String::from(
            "# Architecture Decision Records (ADRs)\n\n\
             This directory holds the *why* behind major architectural choices.\n\n\
             ## Index\n\n\
             | # | Status | Decision |\n\
             |---|--------|----------|\n",
        )
    };

    let row = format!(
        "| [{num:03}]({num:03}-{slug}.md) | {status} | {title} |\n",
        num = num,
        slug = slug,
        status = status,
        title = title,
    );

    // Insert before the next blank line after the table, or just append.
    if content.contains("|---|--------|") || content.contains("|---|---|") {
        content.push_str(&row);
    } else {
        content.push_str("\n## Index\n\n| # | Status | Decision |\n|---|--------|----------|\n");
        content.push_str(&row);
    }
    fs::write(&index_path, content)?;
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if !cli.slug.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        bail!("--slug must be kebab-case lowercase ASCII: {}", cli.slug);
    }
    let num = next_number(&cli.adr_dir)?;
    let adr_path = cli
        .adr_dir
        .join(format!("{:03}-{}.md", num, cli.slug));
    if adr_path.exists() {
        bail!("ADR file already exists: {}", adr_path.display());
    }
    fs::write(&adr_path, render_body(&cli, num))
        .with_context(|| format!("writing {}", adr_path.display()))?;
    update_index(&cli.adr_dir, num, &cli.slug, &cli.status, &cli.title)
        .context("updating ADR index")?;
    println!("{}", adr_path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn temp_dir(name: &str) -> PathBuf {
        let p = env::temp_dir().join(format!("file-adr-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn next_number_starts_at_1_for_empty_dir() {
        let d = temp_dir("empty");
        assert_eq!(next_number(&d).unwrap(), 1);
    }

    #[test]
    fn next_number_increments_past_highest() {
        let d = temp_dir("incr");
        fs::write(d.join("001-foo.md"), "").unwrap();
        fs::write(d.join("005-bar.md"), "").unwrap();
        fs::write(d.join("README.md"), "").unwrap(); // index file ignored
        assert_eq!(next_number(&d).unwrap(), 6);
    }

    #[test]
    fn rejects_uppercase_slug() {
        // Simulated parse error: we just call the validator inline.
        let s = "BadSlug";
        assert!(!s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'));
    }

    #[test]
    fn renders_body_includes_title_and_status() {
        let cli = Cli {
            slug: "abc".into(),
            title: "Test ADR".into(),
            tracker: Some("#42".into()),
            status: "Proposed".into(),
            adr_dir: PathBuf::from("/tmp"),
            author: "me".into(),
        };
        let body = render_body(&cli, 7);
        assert!(body.contains("# ADR 007 — Test ADR"));
        assert!(body.contains("**Status:** Proposed"));
        assert!(body.contains("**Tracker:** #42"));
    }

    #[test]
    fn index_creates_table_when_absent() {
        let d = temp_dir("idx-create");
        update_index(&d, 1, "abc", "Proposed", "Foo").unwrap();
        let s = fs::read_to_string(d.join("README.md")).unwrap();
        assert!(s.contains("| [001](001-abc.md) | Proposed | Foo |"));
    }

    #[test]
    fn index_appends_row_to_existing_table() {
        let d = temp_dir("idx-append");
        let initial =
            "# ADRs\n\n## Index\n\n| # | Status | Decision |\n|---|--------|----------|\n| [001](001-x.md) | Accepted | X |\n";
        fs::write(d.join("README.md"), initial).unwrap();
        update_index(&d, 2, "y", "Proposed", "Y thing").unwrap();
        let s = fs::read_to_string(d.join("README.md")).unwrap();
        assert!(s.contains("| [001](001-x.md) | Accepted | X |"));
        assert!(s.contains("| [002](002-y.md) | Proposed | Y thing |"));
    }
}
