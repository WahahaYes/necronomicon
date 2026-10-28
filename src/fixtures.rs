//! `necro test` (spec: Fixtures): each rule's good and bad trees, run as if
//! each were the repo, with `bad`'s full output kept as `expected.txt`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::config::{Check, Config, Loaded};
use crate::render::full;
use crate::run::{self, Report};
use crate::script::Process;

pub enum Problem {
    /// No `good` (and no shared group `good`), or no `bad`.
    Missing(&'static str),
    /// `good` produced hits or a check failed: the terse lines or the error.
    Good(Vec<String>),
    /// `bad` produced no hits of the rule.
    BadClean,
    BadCheckError(Vec<String>),
    /// `bad`'s output differs from `expected.txt` (absent when it doesn't exist).
    Snapshot {
        expected: Option<String>,
        actual: String,
    },
}

pub struct Outcome {
    pub code: String,
    pub problems: Vec<Problem>,
    pub updated: bool,
}

/// Every file under `dir`, relative to it, sorted.
fn tree(dir: &Path) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&d) else {
            continue;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else if let Some(rel) = crate::files::rel(dir, &p) {
                out.push(rel);
            }
        }
    }
    out.sort();
    out
}

fn run_tree(root: &Path, loaded: &Loaded, tree_dir: &Path, rule: Option<&str>) -> Report {
    let config = &loaded.config;
    let used = |group: &str| {
        config.rules.iter().any(|r| {
            matches!(&r.check, Check::Script { group: g, .. } if g == group)
                && rule.is_none_or(|code| r.code == code || r.group == "necro")
        })
    };
    let mut scripts = BTreeMap::new();
    let mut broken = Vec::new();
    for g in config
        .groups
        .iter()
        .filter(|g| g.command.is_some() && used(&g.name))
    {
        let started = Process::spawn(root, tree_dir, g).and_then(|mut p| p.describe().map(|_| p));
        match started {
            Ok(p) => {
                scripts.insert(g.name.clone(), p);
            }
            Err(f) => broken.push(crate::config::BrokenGroup {
                group: g.name.clone(),
                message: f.message,
                fatal: true,
            }),
        }
    }
    let files = tree(tree_dir);
    let opts = run::Options { given: None, rule };
    run::run(config, &broken, &mut scripts, tree_dir, &files, &opts)
}

fn errors_of(report: &Report) -> Vec<String> {
    let mut lines: Vec<String> = report
        .config_errors
        .iter()
        .map(|e| format!("config error: {}: {}: {}", e.file, e.path, e.message))
        .collect();
    lines.extend(
        report
            .check_errors
            .iter()
            .map(|c| format!("check error: {}: {}", c.group, c.failure.message)),
    );
    lines
}

pub fn test(root: &Path, loaded: &Loaded, update: bool) -> Vec<Outcome> {
    let config: &Config = &loaded.config;
    let base = root.join("necro/fixtures");
    let mut outcomes = Vec::new();
    for r in &config.rules {
        let name = r.code.split_once('/').map_or("", |(_, n)| n);
        let dir = base.join(&r.group).join(name);
        let good: Option<PathBuf> = [dir.join("good"), base.join(&r.group).join("good")]
            .into_iter()
            .find(|p| p.is_dir());
        let bad = Some(dir.join("bad")).filter(|p| p.is_dir());
        // The necro group's own fixtures run every rule.
        let only = (r.group != "necro").then_some(r.code.as_str());
        let mut outcome = Outcome {
            code: r.code.clone(),
            problems: Vec::new(),
            updated: false,
        };
        match &good {
            None => outcome.problems.push(Problem::Missing("good")),
            Some(g) => {
                let report = run_tree(root, loaded, g, only);
                let mut lines = errors_of(&report);
                lines.extend(
                    report.diagnostics.iter().map(|d| {
                        format!("{}:{}:{}: {} {}", d.file, d.line, d.col, d.code, d.summary)
                    }),
                );
                if !lines.is_empty() {
                    outcome.problems.push(Problem::Good(lines));
                }
            }
        }
        match &bad {
            None => outcome.problems.push(Problem::Missing("bad")),
            Some(b) => {
                let report = run_tree(root, loaded, b, only);
                let errors = errors_of(&report);
                if !errors.is_empty() {
                    outcome.problems.push(Problem::BadCheckError(errors));
                } else if !report.diagnostics.iter().any(|d| d.code == r.code) {
                    outcome.problems.push(Problem::BadClean);
                } else {
                    let actual = full::render(&report, true);
                    let path = dir.join("expected.txt");
                    let expected = std::fs::read_to_string(&path)
                        .ok()
                        .map(|s| s.replace("\r\n", "\n"));
                    if expected.as_deref() != Some(actual.as_str()) {
                        if update {
                            outcome.updated = std::fs::write(&path, &actual).is_ok();
                        } else {
                            outcome
                                .problems
                                .push(Problem::Snapshot { expected, actual });
                        }
                    }
                }
            }
        }
        outcomes.push(outcome);
    }
    outcomes
}
