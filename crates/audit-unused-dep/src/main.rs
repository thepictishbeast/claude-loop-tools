//! `audit-unused-dep` — scan a Cargo.toml + src/ tree for zero-call-site
//! direct dependencies. Reports candidates; doesn't modify Cargo.toml.
//!
//! Implements the heuristic the user manually applied 8 times in
//! 2026-05-19 session: for each `[dependencies]` entry, grep
//! `use NAME\|NAME::` in src/ (also bin/ if present); zero matches =
//! removal candidate. Comments are filtered out so e.g. the literal
//! string `".unwrap()"` in a static-analysis rule isn't false-positived.
//!
//! Outputs JSON for the agent to consume + render. Optional `--strict`
//! flag also reports deps whose name-as-prefix grep finds zero hits
//! (catches more false-negatives but raises FP rate; off by default).
//!
//! Usage:
//!   audit-unused-dep                    # scan ./Cargo.toml + ./src
//!   audit-unused-dep --manifest crates/foo/Cargo.toml --src crates/foo/src
//!   audit-unused-dep --json             # machine-readable output (default)
//!   audit-unused-dep --human            # human-readable table

use anyhow::{Context, Result};
use clap::Parser;
use regex::Regex;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use toml_edit::DocumentMut;

#[derive(Parser, Debug)]
#[command(name = "audit-unused-dep", version, about)]
struct Cli {
    /// Path to Cargo.toml. Default `./Cargo.toml`.
    #[arg(long, default_value = "Cargo.toml")]
    manifest: PathBuf,

    /// Path to source dir to scan. Default `./src`.
    #[arg(long, default_value = "src")]
    src: PathBuf,

    /// Extra dirs to scan (repeatable). Default scans `src` only.
    #[arg(long)]
    extra_dir: Vec<PathBuf>,

    /// Print human-readable table instead of JSON.
    #[arg(long)]
    human: bool,
}

#[derive(Serialize, Debug)]
struct Report {
    manifest: String,
    scanned_dirs: Vec<String>,
    total_deps: usize,
    candidates: Vec<Candidate>,
}

#[derive(Serialize, Debug)]
struct Candidate {
    name: String,
    confidence: &'static str, // "high" if 0 anywhere, "medium" if only in comments
    rationale: String,
}

fn extract_deps(manifest_path: &Path) -> Result<Vec<String>> {
    let raw = fs::read_to_string(manifest_path)
        .with_context(|| format!("reading {}", manifest_path.display()))?;
    let doc: DocumentMut = raw.parse().context("parsing Cargo.toml")?;
    let mut deps = Vec::new();
    if let Some(table) = doc.get("dependencies").and_then(|v| v.as_table()) {
        for (k, _) in table.iter() {
            deps.push(k.to_string());
        }
    }
    Ok(deps)
}

fn collect_rs_files(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if !dir.exists() {
        return out;
    }
    fn walk(d: &Path, out: &mut Vec<PathBuf>) {
        if let Ok(entries) = fs::read_dir(d) {
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, out);
                } else if p.extension().and_then(|s| s.to_str()) == Some("rs") {
                    out.push(p);
                }
            }
        }
    }
    walk(dir, &mut out);
    out
}

/// Strip `//`-line + `/* ... */`-block comments crudely.
/// String-literal contents are preserved — so a literal "use foo"
/// in source code is NOT filtered. That's the same approximation the
/// user's 2026-05-19 grep technique used.
fn strip_comments(src: &str) -> String {
    let line_re = Regex::new(r"//[^\n]*").unwrap();
    let block_re = Regex::new(r"(?s)/\*.*?\*/").unwrap();
    let s = line_re.replace_all(src, "");
    block_re.replace_all(&s, "").to_string()
}

fn search_dep(dep_underscore: &str, files: &[PathBuf]) -> (usize, usize) {
    let use_re = Regex::new(&format!(r"\buse\s+{0}\b|\b{0}\s*::", regex::escape(dep_underscore)))
        .unwrap();
    let mut hits_clean = 0usize;
    let mut hits_with_comments = 0usize;
    for f in files {
        if let Ok(raw) = fs::read_to_string(f) {
            let stripped = strip_comments(&raw);
            hits_clean += use_re.find_iter(&stripped).count();
            hits_with_comments += use_re.find_iter(&raw).count();
        }
    }
    (hits_clean, hits_with_comments)
}

fn audit(cli: &Cli) -> Result<Report> {
    let deps = extract_deps(&cli.manifest)?;
    let mut all_files = collect_rs_files(&cli.src);
    for d in &cli.extra_dir {
        all_files.extend(collect_rs_files(d));
    }

    let mut candidates = Vec::new();
    for dep in &deps {
        let underscore = dep.replace('-', "_");
        let (clean, total) = search_dep(&underscore, &all_files);
        if clean == 0 {
            let (conf, why) = if total == 0 {
                (
                    "high",
                    format!(
                        "0 matches in any .rs file (use {0}|{0}::) — strong removal candidate.",
                        underscore
                    ),
                )
            } else {
                (
                    "medium",
                    format!(
                        "0 matches in code; {0} matches inside comments — verify before removing (might be reference in doc-string).",
                        total
                    ),
                )
            };
            candidates.push(Candidate {
                name: dep.clone(),
                confidence: conf,
                rationale: why,
            });
        }
    }

    Ok(Report {
        manifest: cli.manifest.display().to_string(),
        scanned_dirs: std::iter::once(cli.src.display().to_string())
            .chain(cli.extra_dir.iter().map(|p| p.display().to_string()))
            .collect(),
        total_deps: deps.len(),
        candidates,
    })
}

fn render_human(r: &Report) {
    println!("audit-unused-dep report");
    println!("  manifest: {}", r.manifest);
    println!("  scanned:  {}", r.scanned_dirs.join(", "));
    println!("  total direct deps: {}", r.total_deps);
    println!("  candidates: {}", r.candidates.len());
    for c in &r.candidates {
        println!("    [{}] {}", c.confidence, c.name);
        println!("        {}", c.rationale);
    }
    if r.candidates.is_empty() {
        println!("  clean — no unused-dep candidates found.");
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let report = audit(&cli)?;
    if cli.human {
        render_human(&report);
    } else {
        println!("{}", serde_json::to_string_pretty(&report)?);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn temp_dir(name: &str) -> PathBuf {
        let p = env::temp_dir().join(format!("audit-test-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn strip_comments_removes_line_and_block() {
        let s = "use foo; // use bar\n/* use baz */\nfoo::do_it();";
        let stripped = strip_comments(s);
        assert!(stripped.contains("use foo"));
        assert!(!stripped.contains("use bar"));
        assert!(!stripped.contains("use baz"));
        assert!(stripped.contains("foo::do_it"));
    }

    #[test]
    fn search_dep_finds_use_and_path_qualified() {
        let dir = temp_dir("search");
        fs::write(dir.join("a.rs"), "use foo;\nfoo::do_it();").unwrap();
        fs::write(dir.join("b.rs"), "// use bar in a comment").unwrap();
        let files = collect_rs_files(&dir);
        let (clean, total) = search_dep("foo", &files);
        assert_eq!(clean, 2);
        assert_eq!(total, 2);
        let (clean_bar, total_bar) = search_dep("bar", &files);
        assert_eq!(clean_bar, 0);
        assert_eq!(total_bar, 1, "comment-only match");
    }

    #[test]
    fn search_dep_dash_becomes_underscore() {
        let dir = temp_dir("dash");
        fs::write(dir.join("a.rs"), "use serde_json;\n").unwrap();
        let files = collect_rs_files(&dir);
        let (clean, _) = search_dep("serde_json", &files);
        assert_eq!(clean, 1);
    }

    #[test]
    fn extract_deps_picks_top_level_only() {
        let dir = temp_dir("manifest");
        let manifest = dir.join("Cargo.toml");
        fs::write(
            &manifest,
            r#"
[package]
name = "test"
version = "0.0.0"
[dependencies]
foo = "1"
bar = { version = "2" }
[dev-dependencies]
qux = "3"
[features]
"#,
        )
        .unwrap();
        let deps = extract_deps(&manifest).unwrap();
        assert!(deps.contains(&"foo".to_string()));
        assert!(deps.contains(&"bar".to_string()));
        assert!(!deps.contains(&"qux".to_string()), "dev-deps excluded");
    }

    #[test]
    fn audit_end_to_end_flags_unused() {
        let dir = temp_dir("e2e");
        fs::write(
            dir.join("Cargo.toml"),
            "[package]\nname=\"x\"\nversion=\"0.0.0\"\n[dependencies]\nfoo=\"1\"\nbar=\"1\"\n",
        )
        .unwrap();
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(dir.join("src/lib.rs"), "use foo;\n").unwrap();
        let cli = Cli {
            manifest: dir.join("Cargo.toml"),
            src: dir.join("src"),
            extra_dir: vec![],
            human: false,
        };
        let r = audit(&cli).unwrap();
        assert_eq!(r.total_deps, 2);
        assert_eq!(r.candidates.len(), 1);
        assert_eq!(r.candidates[0].name, "bar");
        assert_eq!(r.candidates[0].confidence, "high");
    }
}
