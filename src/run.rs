//! One run: every rule's command, then allows (spec: Commands, Allows).
//! Returns a report for render/.

use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use crate::allows::{self, Allow};
use crate::command::{self, Failure};
use crate::config::{self, Config, Rule};

/// One rule hit with the words that teach it.
pub struct Diagnostic {
    pub rule: String,
    pub file: String,
    /// 0 for a whole-file hit.
    pub line: usize,
    pub col: usize,
    /// The underline's width in code points.
    pub length: usize,
    pub label: String,
    pub message: String,
    pub why: String,
    pub fix: String,
    /// The raw source line, for line > 0.
    pub source: Option<String>,
}

/// One allow that suppressed a hit.
pub struct Allowed {
    pub rule: String,
    pub file: String,
    pub line: usize,
    pub col: usize,
    pub reason: String,
}

/// A run that never got to hits: setup or check.
pub enum Problem {
    Setup(String),
    Check {
        rule: String,
        message: String,
        stderr: Vec<String>,
    },
}

/// Everything one run found: hits, allows and problems.
#[derive(Default)]
pub struct Report {
    pub diagnostics: Vec<Diagnostic>,
    pub allowed: Vec<Allowed>,
    pub problems: Vec<Problem>,
}

impl Report {
    /// 2 with problems, 1 with hits, else 0.
    pub fn exit_code(&self) -> u8 {
        if !self.problems.is_empty() {
            2
        } else if !self.diagnostics.is_empty() {
            1
        } else {
            0
        }
    }
}

/// A hit before allows.
struct Raw<'a> {
    rule: &'a Rule,
    hit: command::Hit,
}

/// Run the selected rules, apply allows, report the rest.
pub fn run(config: &Config, root: &Path, only: Option<&str>) -> Report {
    let mut report = Report::default();
    let selected: Vec<&Rule> = config
        .rules
        .iter()
        .filter(|r| only.is_none_or(|o| r.name == o))
        .collect();
    let outputs: Vec<Result<Vec<String>, Failure>> = std::thread::scope(|s| {
        let handles: Vec<_> = selected
            .iter()
            .map(|r| {
                let command = r.command.as_deref().unwrap_or_default();
                s.spawn(move || command::run(command, root))
            })
            .collect();
        handles
            .into_iter()
            .map(|h| {
                h.join()
                    .unwrap_or_else(|_| Err(Failure::Check("panicked".into(), vec![])))
            })
            .collect()
    });

    let mut raw: Vec<Raw> = Vec::new();
    let mut ran: BTreeSet<&str> = BTreeSet::new();
    for (rule, output) in selected.iter().copied().zip(outputs) {
        match output.and_then(|lines| read_hits(root, &lines)) {
            Ok(hits) => {
                ran.insert(&rule.name);
                raw.extend(hits.into_iter().map(|hit| Raw { rule, hit }));
            }
            Err(Failure::Missing(program)) => report.problems.push(Problem::Setup(format!(
                "{}: {program} isn't on PATH",
                rule.name
            ))),
            Err(Failure::Check(message, stderr)) => report.problems.push(Problem::Check {
                rule: rule.name.clone(),
                message,
                stderr,
            }),
        }
    }

    let mut found = match allows::find(root) {
        Ok(a) => a,
        Err(message) => {
            report.problems.push(Problem::Setup(message));
            vec![]
        }
    };
    let mut used = vec![false; found.len()];
    raw.sort_by(|a, b| {
        (&a.rule.name, &a.hit.path, a.hit.line, a.hit.col).cmp(&(
            &b.rule.name,
            &b.hit.path,
            b.hit.line,
            b.hit.col,
        ))
    });
    let mut kept: Vec<Raw> = Vec::new();
    for r in raw {
        match allow_for(&found, &r) {
            Some(i) => {
                if !used[i] {
                    report.allowed.push(Allowed {
                        rule: r.rule.name.clone(),
                        file: r.hit.path.clone(),
                        line: r.hit.line,
                        col: r.hit.col,
                        reason: found[i].reason.clone().unwrap_or_default(),
                    });
                }
                used[i] = true;
            }
            None => kept.push(r),
        }
    }

    let mut sources = Sources {
        root,
        cache: HashMap::new(),
    };
    let known: BTreeSet<&str> = config.names().into_iter().collect();
    let [reason_rule, unused_rule] = config::builtins();
    for (a, used) in found.iter_mut().zip(&used) {
        if a.reason.is_none() {
            let d = allow_diagnostic(&reason_rule, a, "gives no reason", &mut sources);
            report.diagnostics.push(d);
        }
        if !used && (!known.contains(a.rule.as_str()) || ran.contains(a.rule.as_str())) {
            let d = allow_diagnostic(&unused_rule, a, "suppresses nothing", &mut sources);
            report.diagnostics.push(d);
        }
    }

    for r in kept {
        let d = diagnostic(r, &mut sources);
        report.diagnostics.push(d);
    }

    report
        .diagnostics
        .sort_by(|a, b| (&a.rule, &a.file, a.line, a.col).cmp(&(&b.rule, &b.file, b.line, b.col)));
    report
        .allowed
        .sort_by(|a, b| (&a.file, a.line, a.col, &a.rule).cmp(&(&b.file, b.line, b.col, &b.rule)));
    report
}

/// Parse a command's output; every path must be a file under the root.
fn read_hits(root: &Path, lines: &[String]) -> Result<Vec<command::Hit>, Failure> {
    let mut hits = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let Some(hit) = command::parse(line) else {
            let message = format!("can't read output line {}: {line}", i + 1);
            return Err(Failure::Check(message, vec![]));
        };
        if !Path::new(&hit.path).is_relative() || !root.join(&hit.path).is_file() {
            let message = format!("{} isn't a file", hit.path);
            return Err(Failure::Check(message, vec![]));
        }
        hits.push(hit);
    }
    Ok(hits)
}

/// The allow for a hit: on its line, else the line above; a whole-file hit's
/// on line 1, or 2 after `#!`.
fn allow_for(found: &[Allow], r: &Raw) -> Option<usize> {
    let at = |line: usize| {
        found
            .iter()
            .position(|a| a.file == r.hit.path && a.line == line && a.rule == r.rule.name)
    };
    if r.hit.line == 0 {
        let shebang = found.iter().any(|a| a.file == r.hit.path && a.shebang);
        return at(if shebang { 2 } else { 1 });
    }
    at(r.hit.line).or_else(|| at(r.hit.line - 1))
}

struct Sources<'a> {
    root: &'a Path,
    cache: HashMap<String, Vec<String>>,
}

impl Sources<'_> {
    fn line(&mut self, file: &str, line: usize) -> String {
        let lines = self.cache.entry(file.to_string()).or_insert_with(|| {
            let bytes = std::fs::read(self.root.join(file)).unwrap_or_default();
            String::from_utf8_lossy(&bytes)
                .lines()
                .map(String::from)
                .collect()
        });
        lines.get(line - 1).cloned().unwrap_or_default()
    }
}

/// The underline: `col-endcol`; else the label, when the source has it at
/// `col`; else one caret.
fn diagnostic(r: Raw, sources: &mut Sources) -> Diagnostic {
    let h = r.hit;
    let source = (h.line > 0).then(|| sources.line(&h.path, h.line));
    let length = h
        .endcol
        .map_or(1, |end| (end + 1).saturating_sub(h.col).max(1));
    Diagnostic {
        rule: r.rule.name.clone(),
        file: h.path,
        line: h.line,
        col: h.col,
        length,
        label: h.label,
        message: r.rule.message.clone(),
        why: r.rule.why.clone(),
        fix: r.rule.fix.clone(),
        source,
    }
}

fn allow_diagnostic(rule: &Rule, a: &Allow, what: &str, sources: &mut Sources) -> Diagnostic {
    Diagnostic {
        rule: rule.name.clone(),
        file: a.file.clone(),
        line: a.line,
        col: a.col,
        length: "necro-allow".len(),
        label: a.rule.clone(),
        message: format!("the allow for {} {what}", a.rule),
        why: rule.why.clone(),
        fix: rule.fix.clone(),
        source: Some(sources.line(&a.file, a.line)),
    }
}
