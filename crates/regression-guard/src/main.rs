//! `regression-guard` — generate a Rust `#[test]` skeleton that
//! anchors a refactor commit. The skeleton carries a REGRESSION-GUARD
//! docstring with the commit hash + the detected pattern, so a
//! future contributor reverting the fix sees the failure + the
//! commit-of-origin in the test output without `git blame` spelunking.
//!
//! Detects these refactor patterns from `git show <commit>`:
//!
//!   - `.unwrap()` → `let-else`
//!   - `.unwrap_or_else(|_| panic!())` → `let-Ok-else` + `warn+continue`
//!   - `.expect(...)` → SAFETY annotation or graceful-degrade
//!   - bare `panic!()` → typed-error return
//!   - fallback-sentinel SQL → empty-return + warn
//!   - generic "let mut X = D::default(); X.f = v" → struct update
//!
//! Output: a Rust source snippet the agent pastes into the test
//! module of the touched file. Doesn't try to guess the test body —
//! the agent (or a human) fills in the assertions.
//!
//! Usage:
//!   regression-guard <commit-hash>
//!   regression-guard <commit-hash> --module-path crate::cognition::meta_learner
//!   regression-guard <commit-hash> --test-name min_records_zero_does_not_panic

use anyhow::{Context, Result};
use clap::Parser;
use regex::Regex;
use std::process::Command;

#[derive(Parser, Debug)]
#[command(name = "regression-guard", version, about)]
struct Cli {
    /// Refactor commit to anchor the regression-guard test on.
    commit: String,

    /// Optional explicit pattern name if auto-detect picks wrong.
    /// One of: let-else, let-ok-else, warn-continue, safety-annot,
    /// graceful-degrade, struct-update, error-return.
    #[arg(long)]
    pattern: Option<String>,

    /// Test function name (snake_case). Default auto-generated from
    /// the first hunk's function name + pattern.
    #[arg(long)]
    test_name: Option<String>,

    /// Optional module path to mention in the docstring (e.g.
    /// `crate::cognition::meta_learner`).
    #[arg(long)]
    module_path: Option<String>,

    /// Git working dir override. Default cwd.
    #[arg(long)]
    git_dir: Option<String>,
}

#[derive(Debug, Clone, Copy)]
enum Pattern {
    LetElse,
    LetOkElse,
    WarnContinue,
    SafetyAnnot,
    GracefulDegrade,
    StructUpdate,
    ErrorReturn,
    Unknown,
}

impl Pattern {
    fn from_str(s: &str) -> Self {
        match s {
            "let-else" => Self::LetElse,
            "let-ok-else" => Self::LetOkElse,
            "warn-continue" => Self::WarnContinue,
            "safety-annot" => Self::SafetyAnnot,
            "graceful-degrade" => Self::GracefulDegrade,
            "struct-update" => Self::StructUpdate,
            "error-return" => Self::ErrorReturn,
            _ => Self::Unknown,
        }
    }

    fn explanation(&self) -> &'static str {
        match self {
            Self::LetElse => "`.unwrap()` → `let Some(x) = ... else { return None };` — defends against the empty/None pathological case the unwrap would have panicked on.",
            Self::LetOkElse => "`.unwrap_or_else(|_| panic!(...))` → `let Ok(x) = ... else { warn!(...); return ... };` — replaces panic with graceful degrade + observability.",
            Self::WarnContinue => "`if let Ok(x) = stream.next() { ... }` (silent skip) → explicit `match` with `warn!()` + `continue` on Err — surfaces malformed input in logs.",
            Self::SafetyAnnot => "Added `// SAFETY:` annotation to an `unsafe` or `.unwrap()` that's provably correct but lacked the AVP-2-required justification block.",
            Self::GracefulDegrade => "`.expect(\"impossible\")` → return-Err / return-empty / return-sentinel — replaces panic on an \"impossible\" branch with a recoverable path.",
            Self::StructUpdate => "`let mut x = T::default(); x.f = v;` → struct-update syntax `T { f: v, ..Default::default() }` — clippy `field_reassign_with_default`.",
            Self::ErrorReturn => "Bare `panic!()` → typed error return — caller can `?`-propagate instead of crashing.",
            Self::Unknown => "Pattern not auto-detected. Edit the docstring to describe the refactor manually.",
        }
    }
}

fn auto_detect(diff: &str) -> Pattern {
    // Order matters — more specific patterns first.
    // Strip the leading +/- so the regexes don't have to model it.
    let added_lines: Vec<&str> = diff
        .lines()
        .filter(|l| l.starts_with('+') && !l.starts_with("+++"))
        .map(|l| &l[1..])
        .collect();
    let removed_lines: Vec<&str> = diff
        .lines()
        .filter(|l| l.starts_with('-') && !l.starts_with("---"))
        .map(|l| &l[1..])
        .collect();
    let added = added_lines.join("\n");
    let removed = removed_lines.join("\n");

    if Regex::new(r"\.unwrap_or_else\(\|_\|\s*panic!").unwrap().is_match(&removed)
        && Regex::new(r"warn!\s*\(|let Ok\(.*\) =").unwrap().is_match(&added)
    {
        return Pattern::LetOkElse;
    }
    if Regex::new(r"if let Ok\(").unwrap().is_match(&removed)
        && Regex::new(r"\.flatten\(\)|match .* \{\s*Ok\(.*\)\s*=>").unwrap().is_match(&added)
    {
        return Pattern::WarnContinue;
    }
    if Regex::new(r"\.expect\(").unwrap().is_match(&removed)
        && (added.contains("ok_or_else") || added.contains("?;") || added.contains("Err(") || added.contains("else {"))
    {
        return Pattern::GracefulDegrade;
    }
    if Regex::new(r"\.unwrap\(\)").unwrap().is_match(&removed)
        && Regex::new(r"let Some\(.*\) = .* else").unwrap().is_match(&added)
    {
        return Pattern::LetElse;
    }
    if added.contains("// SAFETY:") {
        return Pattern::SafetyAnnot;
    }
    if Regex::new(r"::default\(\);\s*\n\s*\w+\.\w+\s*=").unwrap().is_match(&removed)
        && Regex::new(r"\.\.Default::default\(\)").unwrap().is_match(&added)
    {
        return Pattern::StructUpdate;
    }
    if removed.contains("panic!(") && added.contains("return Err(") {
        return Pattern::ErrorReturn;
    }
    Pattern::Unknown
}

fn first_changed_fn(diff: &str) -> Option<String> {
    let re = Regex::new(r"(?m)^\+\s*(?:pub\s+)?(?:async\s+)?fn\s+(\w+)").unwrap();
    re.captures(diff).map(|c| c[1].to_string())
}

fn git_show(commit: &str, git_dir: Option<&str>) -> Result<(String, String)> {
    let mut cmd = Command::new("git");
    if let Some(d) = git_dir {
        cmd.args(["-C", d]);
    }
    let subject_out = cmd
        .args(["show", "-s", "--format=%s", commit])
        .output()
        .context("git show -s")?;
    if !subject_out.status.success() {
        anyhow::bail!(
            "git show -s failed: {}",
            String::from_utf8_lossy(&subject_out.stderr)
        );
    }
    let subject = String::from_utf8_lossy(&subject_out.stdout).trim().to_string();

    let mut cmd2 = Command::new("git");
    if let Some(d) = git_dir {
        cmd2.args(["-C", d]);
    }
    let diff_out = cmd2
        .args(["show", "--no-color", "--format=", commit])
        .output()
        .context("git show diff")?;
    let diff = String::from_utf8_lossy(&diff_out.stdout).to_string();
    Ok((subject, diff))
}

fn render(commit_short: &str, subject: &str, pattern: Pattern, test_name: &str, module_path: Option<&str>) -> String {
    let module_line = module_path
        .map(|m| format!("    /// Module: `{m}`\n"))
        .unwrap_or_default();
    format!(
        r#"    /// REGRESSION-GUARD — commit `{commit_short}`
    ///
    /// Subject: {subject}
    ///
    /// Pattern: {pattern:?}
    ///
    /// {explanation}
    ///
{module_line}    /// If this test fails, someone reverted the refactor or
    /// changed the invariant it depends on. Inspect the commit
    /// before "fixing" the test.
    #[test]
    fn {test_name}() {{
        // TODO: exercise the pathological input the refactor defends against.
        // Examples per pattern:
        //   let-else        → empty / None input that the prior unwrap would panic on
        //   let-ok-else     → malformed input that the prior unwrap_or_else(panic) would crash on
        //   warn-continue   → mixed valid+invalid stream — verify valid prefix is processed
        //   safety-annot    → exercise the documented invariant directly
        //   graceful-degrade→ the "impossible" branch input
        //   struct-update   → verify default-equiv behaviour
        //   error-return    → verify Err is propagated, not panicked
        //
        // Use #[should_panic] sparingly — prefer asserting the
        // post-refactor behaviour (no panic) explicitly.
    }}
"#,
        commit_short = commit_short,
        subject = subject,
        pattern = pattern,
        explanation = pattern.explanation(),
        module_line = module_line,
        test_name = test_name,
    )
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let (subject, diff) = git_show(&cli.commit, cli.git_dir.as_deref())?;
    let commit_short = if cli.commit.len() >= 7 { &cli.commit[..7] } else { &cli.commit };
    let pattern = match cli.pattern.as_deref() {
        Some(p) => Pattern::from_str(p),
        None => auto_detect(&diff),
    };
    let test_name = cli.test_name.unwrap_or_else(|| {
        let base = first_changed_fn(&diff).unwrap_or_else(|| "refactor".into());
        format!("regression_guard_{base}_{}", commit_short)
    });
    let snippet = render(commit_short, &subject, pattern, &test_name, cli.module_path.as_deref());
    print!("{}", snippet);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_let_ok_else_from_diff() {
        let diff = r#"
-    let x = thing.unwrap_or_else(|_| panic!("boom"));
+    let Ok(x) = thing else {
+        warn!("skipped");
+        return;
+    };
"#;
        assert!(matches!(auto_detect(diff), Pattern::LetOkElse));
    }

    #[test]
    fn detects_let_else_from_diff() {
        let diff = r#"
-    let first = window.first().unwrap();
+    let Some(first) = window.first() else { return None; };
"#;
        assert!(matches!(auto_detect(diff), Pattern::LetElse));
    }

    #[test]
    fn detects_struct_update_from_diff() {
        let diff = r#"
-    let mut cfg = Config::default();
-    cfg.x = 5;
+    let cfg = Config { x: 5, ..Default::default() };
"#;
        assert!(matches!(auto_detect(diff), Pattern::StructUpdate));
    }

    #[test]
    fn detects_safety_annot() {
        let diff = r#"
+    // SAFETY: bounds checked above.
+    let x = unsafe { *ptr };
"#;
        assert!(matches!(auto_detect(diff), Pattern::SafetyAnnot));
    }

    #[test]
    fn unknown_when_no_pattern_matches() {
        let diff = "// just a comment change\n";
        assert!(matches!(auto_detect(diff), Pattern::Unknown));
    }

    #[test]
    fn first_changed_fn_finds_pub_fn() {
        let diff = "+    pub fn do_thing(x: i32) -> u32 { ... }\n";
        assert_eq!(first_changed_fn(diff).as_deref(), Some("do_thing"));
    }

    #[test]
    fn first_changed_fn_finds_plain_fn() {
        let diff = "+fn helper() -> bool { true }\n";
        assert_eq!(first_changed_fn(diff).as_deref(), Some("helper"));
    }

    #[test]
    fn render_includes_commit_subject_pattern() {
        let snippet = render(
            "deadbee",
            "fix: replace unwrap with let-else",
            Pattern::LetElse,
            "regression_guard_foo_deadbee",
            Some("crate::foo"),
        );
        assert!(snippet.contains("commit `deadbee`"));
        assert!(snippet.contains("fix: replace unwrap with let-else"));
        assert!(snippet.contains("regression_guard_foo_deadbee"));
        assert!(snippet.contains("crate::foo"));
    }
}
