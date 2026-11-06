//! One run: every rule's command, then allows (spec: Commands, Allows).
//! Returns a report for render/.

use std::collections::{BTreeSet, HashMap};
use std::path::Path;
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant};

use crate::allows::{self, Allow};
use crate::baseline::Baseline;
use crate::command::{self, Failure};
use crate::config::{self, Config, Rule};
use crate::render;

/// How long a command runs before necro says so; it isn't killed (spec:
/// Commands).
const LONG_RUN: Duration = Duration::from_secs(60);

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
    /// Snapshot entries no observed hit consumed: a warning, never a failure.
    pub stale_baseline: usize,
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

/// Run the selected rules, apply allows and the snapshot, report the rest.
pub fn run(config: &Config, root: &Path, only: Option<&str>, given: Option<&[String]>) -> Report {
    let mut report = run_observed(config, root, only, given);
    let path = config.baseline_path(root);
    match Baseline::load(&path) {
        // No snapshot (or an empty one): nothing to silence.
        Ok(baseline) if baseline.is_empty() => {}
        Ok(baseline) => {
            // A filtered run matches only its own slice: other rules'
            // leftovers aren't stale, they just didn't run.
            let baseline = match only {
                Some(rule) => baseline.scoped(rule),
                None => baseline,
            };
            // A narrowed run judges stale entries only in its given files.
            let (kept, stale) = baseline
                .match_hits_counting(std::mem::take(&mut report.diagnostics), |path| {
                    given.is_none_or(|g| g.iter().any(|f| f == path))
                });
            report.diagnostics = kept;
            report.stale_baseline = stale;
        }
        Err(e) => report.problems.push(Problem::Setup(e.to_string())),
    }
    report
}

/// One run through allows, without the snapshot: every rule's command, then
/// allows (spec: Commands, Allows). `--write-baseline` snapshots this.
pub fn run_observed(
    config: &Config,
    root: &Path,
    only: Option<&str>,
    given: Option<&[String]>,
) -> Report {
    let mut report = Report::default();
    let selected: Vec<&Rule> = config
        .rules
        .iter()
        .filter(|r| only.is_none_or(|o| r.name == o))
        .filter(|r| given.is_none_or(|g| r.runs_on(g)))
        .collect();
    let list = match given.map(GivenList::write).transpose() {
        Ok(list) => list,
        Err(message) => {
            report.problems.push(Problem::Setup(message));
            return report;
        }
    };
    let list_path = list.as_ref().map(|l| l.path.as_path());
    let outputs: Vec<Result<Vec<String>, Failure>> = std::thread::scope(|s| {
        let (done, finished) = mpsc::channel();
        let handles: Vec<_> = selected
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let words = narrowed(r.words(), list_path);
                let done = done.clone();
                s.spawn(move || {
                    let output = command::run(words, root);
                    // necro-allow rust/swallowed-error: the watcher stops listening once it has spoken
                    let _ = done.send(i);
                    output
                })
            })
            .collect();
        drop(done);
        let names: Vec<&str> = selected.iter().map(|r| r.name.as_str()).collect();
        s.spawn(move || watch(&names, &finished, LONG_RUN, render::still_running));
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
                // An excluded file is outside the rule, or the whole run, in every run.
                raw.extend(
                    hits.into_iter()
                        .filter(|hit| !rule.excludes(&hit.path) && !config.excludes(&hit.path))
                        .map(|hit| Raw { rule, hit }),
                );
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
        // The top-level exclude's files hold no allows, only quoted text.
        Ok(a) => a
            .into_iter()
            .filter(|a| !config.excludes(&a.file))
            .collect(),
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
        // A narrowed run checks allows only in its given files.
        if given.is_some_and(|g| !g.contains(&a.file)) {
            continue;
        }
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

/// The finders that search files, which a narrowed run hands its list.
const FILE_FINDERS: &[&str] = &["grep", "rust-docs", "gdscan", "gdtoolkit"];

/// A rule's command words; in a narrowed run, necro's file finders get
/// `--files-from <list>` after their name (spec: Narrowed runs).
fn narrowed(mut words: Vec<String>, list: Option<&Path>) -> Vec<String> {
    if let Some(list) = list
        && words.first().is_some_and(|w| w == "necro")
        && words
            .get(1)
            .is_some_and(|w| FILE_FINDERS.contains(&w.as_str()))
    {
        let at = words.len().min(2);
        words.splice(
            at..at,
            ["--files-from".to_string(), list.display().to_string()],
        );
    }
    words
}

/// The given files written one per line to a temporary file, removed when
/// the run ends.
struct GivenList {
    path: std::path::PathBuf,
}

impl GivenList {
    fn write(given: &[String]) -> Result<GivenList, String> {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.subsec_nanos());
        let name = format!("necro-files-{}-{nanos}", std::process::id());
        let path = std::env::temp_dir().join(name);
        let mut text = given.join("\n");
        text.push('\n');
        std::fs::write(&path, text)
            .map_err(|e| format!("can't write the file list {}: {e}", path.display()))?;
        Ok(GivenList { path })
    }
}

impl Drop for GivenList {
    fn drop(&mut self) {
        // necro-allow rust/swallowed-error: a leftover list in the temp folder is harmless
        let _ = std::fs::remove_file(&self.path);
    }
}

/// Wait for every rule's command; at `after`, name the ones still running,
/// once, and stop watching.
fn watch(
    names: &[&str],
    finished: &Receiver<usize>,
    after: Duration,
    say: impl FnOnce(&[&str], u64),
) {
    let deadline = Instant::now() + after;
    let mut running: BTreeSet<usize> = (0..names.len()).collect();
    while !running.is_empty() {
        match finished.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok(i) => {
                running.remove(&i);
            }
            Err(RecvTimeoutError::Timeout) => {
                let late: Vec<&str> = running.iter().map(|&i| names[i]).collect();
                return say(&late, after.as_secs());
            }
            Err(RecvTimeoutError::Disconnected) => return,
        }
    }
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
        let at = root.join(&hit.path);
        let is_link = at
            .symlink_metadata()
            .is_ok_and(|m| m.file_type().is_symlink());
        if !Path::new(&hit.path).is_relative() || !(at.is_file() || is_link) {
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

/// What `necro --fix` did (spec: Fixes): the run after the fixes, the hits
/// each fixed rule cleared, and how many files changed.
pub struct Fixed {
    pub report: Report,
    pub fixed: Vec<(String, usize)>,
    pub changed: usize,
}

/// Run, fix what the reported hits' rules can, one rule at a time, then run again.
pub fn fix(config: &Config, root: &Path, only: Option<&str>, given: Option<&[String]>) -> Fixed {
    let before = run(config, root, only, given);
    let mut problems: Vec<Problem> = Vec::new();
    let mut changed: BTreeSet<String> = BTreeSet::new();
    // What a fix changed: files git sees as modified or new, and the hit
    // files, compared before and after every fix ran.
    let hit_files: Vec<String> = before.diagnostics.iter().map(|d| d.file.clone()).collect();
    let before_files = worktree_changes(root, &hit_files);
    let mut counts: Vec<(&Rule, usize)> = Vec::new();
    for rule in &config.rules {
        let Some(fixer) = rule.fixer() else { continue };
        let hits: Vec<&Diagnostic> = before
            .diagnostics
            .iter()
            .filter(|d| d.rule == rule.name)
            .collect();
        if hits.is_empty() {
            continue;
        }
        counts.push((rule, hits.len()));
        let mut files: Vec<String> = hits.iter().map(|d| d.file.clone()).collect();
        files.dedup();
        let at: Vec<String> = hits
            .iter()
            .map(|d| format!("{}:{}:{}", d.file, d.line, d.col))
            .collect();
        let list = match GivenList::write(&at) {
            Ok(list) => list,
            Err(message) => {
                problems.push(Problem::Setup(message));
                continue;
            }
        };
        let words = match fixer {
            config::Fixer::Finder => {
                let mut words = rule.words();
                let at = words.len().min(2);
                words.splice(
                    at..at,
                    [
                        "--write".to_string(),
                        "--only-at".to_string(),
                        list.path.display().to_string(),
                    ],
                );
                words
            }
            config::Fixer::Command => rule.autofix_words(&files),
        };
        match command::run(words, root) {
            Ok(_) => {}
            Err(Failure::Missing(program)) => problems.push(Problem::Setup(format!(
                "{}: {program} isn't on PATH",
                rule.name
            ))),
            Err(Failure::Check(message, stderr)) => problems.push(Problem::Check {
                rule: rule.name.clone(),
                message: format!("its fix failed: {message}"),
                stderr,
            }),
        }
    }
    let after = worktree_changes(root, &hit_files);
    for (file, content) in &after {
        if before_files.get(file) != Some(content) {
            changed.insert(file.clone());
        }
    }
    let mut report = run(config, root, only, given);
    report.problems.splice(0..0, problems);
    let fixed = counts
        .into_iter()
        .map(|(rule, n)| {
            let left = report
                .diagnostics
                .iter()
                .filter(|d| d.rule == rule.name)
                .count();
            (rule.name.clone(), n.saturating_sub(left))
        })
        .filter(|(_, n)| *n > 0)
        .collect();
    Fixed {
        report,
        fixed,
        changed: changed.len(),
    }
}

/// How many of the hits a fix could clear: the ones whose rule has a fixer.
pub fn fixable(config: &Config, diagnostics: &[Diagnostic]) -> usize {
    diagnostics
        .iter()
        .filter(|d| {
            config
                .rules
                .iter()
                .any(|r| r.name == d.rule && r.fixer().is_some())
        })
        .count()
}

/// The working tree's modified and untracked files, plus `extra`, each with
/// its content (a symlink's target; `None` when it doesn't exist).
fn worktree_changes(root: &Path, extra: &[String]) -> HashMap<String, Option<Vec<u8>>> {
    let listed = crate::files::git()
        .current_dir(root)
        .args(["ls-files", "-z", "-m", "-o", "--exclude-standard"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .unwrap_or_default();
    listed
        .split('\0')
        .filter(|p| !p.is_empty())
        .map(String::from)
        .chain(extra.iter().cloned())
        .map(|p| {
            let at = root.join(&p);
            let content = match std::fs::read_link(&at) {
                Ok(target) => Some(target.to_string_lossy().into_owned().into_bytes()),
                Err(_) => std::fs::read(&at).ok(),
            };
            (p, content)
        })
        .collect()
}
