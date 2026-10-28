//! One run: rules over files, then allows and severity (spec: Scope and
//! parameters, Check protocol, Allows). Returns a report for render/.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use crate::checks::{self, Allow, RawHit, Value};
use crate::config::{
    BrokenGroup, Check, Config, ConfigError, Engine, ParamValue, Rule, Scope, Severity, Wording,
    placeholders,
};
use crate::lang::{self, Source};
use crate::script::{Failure, Job, Process};

pub struct Diagnostic {
    pub code: String,
    pub severity: Severity,
    pub file: String,
    pub line: usize,
    pub col: usize,
    pub length: usize,
    pub summary: String,
    pub why: String,
    pub help: String,
    pub label: Option<String>,
    /// The raw source line, for line > 0.
    pub source: Option<String>,
}

pub struct Allowed {
    pub code: String,
    pub file: String,
    pub line: usize,
    pub reason: String,
}

pub struct CheckError {
    pub group: String,
    pub command: String,
    pub failure: Failure,
    /// An `error` rule uses the check, so the run fails.
    pub fatal: bool,
}

#[derive(Default)]
pub struct Report {
    pub diagnostics: Vec<Diagnostic>,
    pub allowed: Vec<Allowed>,
    pub warnings: Vec<String>,
    pub check_errors: Vec<CheckError>,
    pub config_errors: Vec<ConfigError>,
}

impl Report {
    pub fn exit_code(&self) -> u8 {
        if !self.config_errors.is_empty() || self.check_errors.iter().any(|c| c.fatal) {
            2
        } else if self
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
        {
            1
        } else {
            0
        }
    }
}

pub struct Options<'a> {
    /// Files named on the command line (root-relative), or all in scope.
    pub given: Option<&'a [String]>,
    pub rule: Option<&'a str>,
}

struct Sources<'a> {
    root: &'a Path,
    config: &'a Config,
    cache: HashMap<String, Source>,
}

impl Sources<'_> {
    fn get(&mut self, file: &str) -> &Source {
        if !self.cache.contains_key(file) {
            let bytes = std::fs::read(self.root.join(file)).unwrap_or_default();
            let text = String::from_utf8_lossy(&bytes);
            let src = lang::scan(&text, self.config.lang(file));
            self.cache.insert(file.to_string(), src);
        }
        &self.cache[file]
    }
}

/// The rule's params for one file: the rule's, then any `where` entry matching it.
pub fn params_for(rule: &Rule, file: &str) -> BTreeMap<String, ParamValue> {
    let mut params = rule.params.clone();
    for w in rule.wheres.iter().filter(|w| w.matches(file)) {
        params.extend(w.params.clone());
    }
    params
}

pub fn in_scope(rule: &Rule, file: &str) -> bool {
    rule.files.is_match(file) && !rule.ignores.is_match(file)
}

/// `files` are every file in the run, relative to `data_root` (the root, or a fixture tree).
pub fn run(
    config: &Config,
    broken: &[BrokenGroup],
    scripts: &mut BTreeMap<String, Process>,
    data_root: &Path,
    files: &[String],
    opts: &Options,
) -> Report {
    let mut report = Report::default();
    let scope: Vec<&String> = files
        .iter()
        .filter(|f| config.files.is_match(f.as_str()) && !config.ignores.is_match(f.as_str()))
        .collect();
    let targets: Vec<&String> = match opts.given {
        Some(given) => scope
            .iter()
            .copied()
            .filter(|f| given.contains(f))
            .collect(),
        None => scope.clone(),
    };
    let selected: Vec<usize> = (0..config.rules.len())
        .filter(|&i| {
            opts.rule
                .is_none_or(|r| config.rules[i].code == r || config.rules[i].group == "necro")
        })
        .collect();
    let mut sources = Sources {
        root: data_root,
        config,
        cache: HashMap::new(),
    };
    let mut raw: Vec<(usize, String, RawHit)> = Vec::new();

    // Script jobs: identical (check, params, files) from different rules are sent once.
    let mut jobs: BTreeMap<String, Vec<(Job, Vec<usize>)>> = BTreeMap::new();
    for &r in &selected {
        let rule = &config.rules[r];
        let pool = if rule.decl.scope == Scope::Project {
            &scope
        } else {
            &targets
        };
        let rule_files: Vec<&String> = pool.iter().copied().filter(|f| in_scope(rule, f)).collect();
        match &rule.check {
            Check::Regex { re, raw_line } => {
                for f in rule_files {
                    for h in checks::regex(sources.get(f), re, *raw_line) {
                        raw.push((r, f.clone(), h));
                    }
                }
            }
            Check::Engine(Engine::AllowReason | Engine::UnusedAllow) => {}
            Check::Engine(engine) => {
                for f in rule_files {
                    let params = params_for(rule, f);
                    let hits = match engine {
                        Engine::Require => checks::require(sources.get(f), &params),
                        _ => {
                            let Some(lang) = config.lang(f) else {
                                unknown_language(&mut report, rule, f);
                                continue;
                            };
                            let src = sources.get(f);
                            if *engine == Engine::CommentBlocks {
                                checks::comment_blocks(src, lang, &params)
                            } else {
                                checks::doc_above(src, lang, &params)
                            }
                        }
                    };
                    raw.extend(hits.into_iter().map(|h| (r, f.clone(), h)));
                }
            }
            Check::Script { group, name } => {
                if broken.iter().any(|b| &b.group == group) {
                    continue;
                }
                let mut by_params: Vec<(BTreeMap<String, ParamValue>, Vec<String>)> = Vec::new();
                for f in rule_files {
                    let p = params_for(rule, f);
                    match by_params.iter_mut().find(|(q, _)| *q == p) {
                        Some((_, fs)) => fs.push(f.clone()),
                        None => by_params.push((p, vec![f.clone()])),
                    }
                }
                let group_jobs = jobs.entry(group.clone()).or_default();
                for (params, fs) in by_params {
                    match group_jobs
                        .iter_mut()
                        .find(|(j, _)| j.check == *name && j.params == params && j.files == fs)
                    {
                        Some((_, rules)) => rules.push(r),
                        None => {
                            let id = (group_jobs.len() + 1).to_string();
                            group_jobs.push((
                                Job {
                                    id,
                                    check: name.clone(),
                                    files: fs,
                                    params,
                                },
                                vec![r],
                            ));
                        }
                    }
                }
            }
        }
    }

    for b in broken {
        let failure = Failure {
            message: b.message.clone(),
            stderr: vec![],
        };
        push_check_error(&mut report, config, &b.group, failure);
    }
    let in_scope_set: BTreeSet<&str> = scope.iter().map(|s| s.as_str()).collect();
    for (group, process) in std::mem::take(scripts) {
        let group_jobs = jobs.remove(&group).unwrap_or_default();
        let list: Vec<Job> = group_jobs
            .iter()
            .map(|(j, _)| Job {
                id: j.id.clone(),
                check: j.check.clone(),
                files: j.files.clone(),
                params: j.params.clone(),
            })
            .collect();
        let result = process.check(&list).and_then(|resp| {
            if let Some(e) = resp.errors.first() {
                return Err(Failure {
                    message: format!("it reported an error: {e}"),
                    stderr: vec![],
                });
            }
            let mut out = Vec::new();
            for (job, rules) in &group_jobs {
                let hits = resp.results.get(&job.id).ok_or_else(|| Failure {
                    message: format!("no result for job {}", job.id),
                    stderr: vec![],
                })?;
                for h in hits {
                    validate_hit(config, rules, job, h, &in_scope_set).map_err(|m| Failure {
                        message: m,
                        stderr: vec![],
                    })?;
                    for &r in rules {
                        let hit = RawHit {
                            line: h.line,
                            col: h.col,
                            length: h.length,
                            values: h.values.clone(),
                        };
                        out.push((r, h.file.clone(), hit));
                    }
                }
            }
            report
                .warnings
                .extend(resp.warnings.iter().map(|w| format!("{group}: {w}")));
            Ok(out)
        });
        match result {
            Ok(hits) => raw.extend(hits),
            Err(failure) => push_check_error(&mut report, config, &group, failure),
        }
    }

    apply_allows(config, &selected, &targets, &mut sources, raw, &mut report);
    report.diagnostics.sort_by(|a, b| {
        (a.severity, &a.code, &a.file, a.line, a.col)
            .cmp(&(b.severity, &b.code, &b.file, b.line, b.col))
    });
    report
        .allowed
        .sort_by(|a, b| (&a.file, a.line, &a.code).cmp(&(&b.file, b.line, &b.code)));
    report
}

fn unknown_language(report: &mut Report, rule: &Rule, file: &str) {
    let ext = lang::extension(file);
    let check = match rule.check {
        Check::Engine(Engine::CommentBlocks) => "engine.comment-blocks",
        _ => "engine.doc-above",
    };
    let name = rule.code.split_once('/').map_or("", |(_, n)| n);
    let path = format!("rules.{name}.check");
    if report
        .config_errors
        .iter()
        .any(|e| e.path == path && e.file == rule.file)
    {
        return;
    }
    report.config_errors.push(ConfigError {
        file: rule.file.clone(),
        pos: rule.pos,
        path,
        message: format!("{check} doesn't know `{ext}` files (e.g. {file}); narrow the rule's files or add `{ext}` under languages"),
    });
}

fn push_check_error(report: &mut Report, config: &Config, group: &str, failure: Failure) {
    let users = config
        .rules
        .iter()
        .filter(|r| matches!(&r.check, Check::Script { group: g, .. } if g == group));
    let fatal = users.into_iter().any(|r| {
        r.severity == Severity::Error
            || r.wheres.iter().any(|w| w.severity == Some(Severity::Error))
    });
    let command = config
        .groups
        .iter()
        .find(|g| g.name == group)
        .and_then(|g| g.command.as_ref())
        .map(|c| c.join(" "))
        .unwrap_or_default();
    report.check_errors.push(CheckError {
        group: group.to_string(),
        command,
        failure,
        fatal,
    });
}

fn validate_hit(
    config: &Config,
    rules: &[usize],
    job: &Job,
    h: &crate::script::ScriptHit,
    scope: &BTreeSet<&str>,
) -> Result<(), String> {
    let rule = &config.rules[rules[0]];
    let inside = job.files.contains(&h.file) || (h.line == 0 && scope.contains(h.file.as_str()));
    if !inside {
        return Err(format!(
            "job {} returned a hit in {}, which it wasn't sent",
            job.id, h.file
        ));
    }
    for k in h.values.keys() {
        if !rule.decl.values.contains(k) {
            return Err(format!(
                "job {} returned value `{k}`, which `{}` doesn't declare",
                job.id, job.check
            ));
        }
    }
    for &r in rules {
        let rule = &config.rules[r];
        let (_, w) = effective(rule, &h.file);
        for t in [&w.summary, &w.why, &w.help, &w.label]
            .into_iter()
            .flatten()
        {
            for name in placeholders(t) {
                if rule.decl.values.contains(&name) && !h.values.contains_key(&name) {
                    return Err(format!(
                        "job {} returned a hit without value `{name}`, which {}'s wording uses",
                        job.id, rule.code
                    ));
                }
            }
        }
    }
    Ok(())
}

/// The severity and wording that apply to a file: the rule's, with `where` overrides.
fn effective(rule: &Rule, file: &str) -> (Severity, Wording) {
    let mut severity = rule.severity;
    let mut w = rule.wording.clone();
    for entry in rule.wheres.iter().filter(|e| e.matches(file)) {
        severity = entry.severity.unwrap_or(severity);
        let o = &entry.wording;
        w.summary = o.summary.clone().or(w.summary);
        w.why = o.why.clone().or(w.why);
        w.help = o.help.clone().or(w.help);
        w.label = o.label.clone().or(w.label);
    }
    (severity, w)
}

fn apply_allows(
    config: &Config,
    selected: &[usize],
    targets: &[&String],
    sources: &mut Sources,
    raw: Vec<(usize, String, RawHit)>,
    report: &mut Report,
) {
    let codes: BTreeSet<&str> = config.rules.iter().map(|r| r.code.as_str()).collect();
    let ran: BTreeSet<&str> = selected
        .iter()
        .map(|&r| config.rules[r].code.as_str())
        .collect();
    let find = |engine: Engine| {
        selected
            .iter()
            .copied()
            .find(|&r| matches!(config.rules[r].check, Check::Engine(e) if e == engine))
    };
    let (reason_rule, unused_rule) = (find(Engine::AllowReason), find(Engine::UnusedAllow));

    let mut allows: BTreeMap<String, Vec<(Allow, bool)>> = BTreeMap::new();
    let mut shebang: BTreeSet<String> = BTreeSet::new();
    for f in targets {
        if config.lang(f).is_none() {
            continue;
        }
        let src = sources.get(f);
        if src.lines.first().is_some_and(|l| l.starts_with("#!")) {
            shebang.insert(f.to_string());
        }
        let found = checks::allows(src);
        if !found.is_empty() {
            allows.insert(
                f.to_string(),
                found.into_iter().map(|a| (a, false)).collect(),
            );
        }
    }

    let mut kept = Vec::new();
    for (r, file, hit) in raw {
        let code = &config.rules[r].code;
        let lines: Vec<usize> = if hit.line == 0 {
            vec![if shebang.contains(&file) { 2 } else { 1 }]
        } else {
            vec![hit.line, hit.line - 1]
        };
        // The hit's own line first, then the line above.
        let found = allows.get_mut(&file).and_then(|list| {
            let at = lines.iter().find_map(|l| {
                list.iter()
                    .position(|(a, _)| &a.code == code && a.line == *l)
            })?;
            list.get_mut(at)
        });
        match found {
            Some((allow, used)) => {
                if !*used {
                    report.allowed.push(Allowed {
                        code: code.clone(),
                        file: file.clone(),
                        line: hit.line,
                        reason: allow.reason.clone().unwrap_or_default(),
                    });
                }
                *used = true;
            }
            None => kept.push((r, file, hit)),
        }
    }
    for (file, list) in &allows {
        for (allow, used) in list {
            let value = BTreeMap::from([("code".to_string(), Value::Str(allow.code.clone()))]);
            let hit = || RawHit {
                line: allow.line,
                col: allow.col,
                length: allow.length,
                values: value.clone(),
            };
            if allow.reason.is_none()
                && let Some(r) = reason_rule
                && in_scope(&config.rules[r], file)
            {
                kept.push((r, file.clone(), hit()));
            }
            let known = codes.contains(allow.code.as_str());
            let stale = !used && (!known || ran.contains(allow.code.as_str()));
            if stale
                && let Some(r) = unused_rule
                && in_scope(&config.rules[r], file)
            {
                kept.push((r, file.clone(), hit()));
            }
        }
    }
    for (r, file, hit) in kept {
        let d = diagnostic(&config.rules[r], &file, hit, sources);
        report.diagnostics.push(d);
    }
}

pub fn render_value(v: &Value) -> String {
    match v {
        Value::Str(s) => s.clone(),
        Value::Number(n) => number(*n),
    }
}

pub fn number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

pub fn render_param(v: &ParamValue) -> String {
    match v {
        ParamValue::Str(s) => s.clone(),
        ParamValue::Number(n) => number(*n),
        ParamValue::Bool(b) => b.to_string(),
        ParamValue::StrList(l) => l.join(", "),
        ParamValue::StrMap(_) => String::new(),
    }
}

pub fn fill(
    template: &str,
    values: &BTreeMap<String, Value>,
    params: &BTreeMap<String, ParamValue>,
) -> String {
    let mut out = String::new();
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        let Some(end) = rest[start..].find("}}") else {
            break;
        };
        out.push_str(&rest[..start]);
        let name = rest[start + 2..start + end].trim();
        let text = values
            .get(name)
            .map(render_value)
            .or_else(|| params.get(name).map(render_param))
            .unwrap_or_default();
        out.push_str(&text);
        rest = &rest[start + end + 2..];
    }
    out.push_str(rest);
    out
}

fn diagnostic(rule: &Rule, file: &str, hit: RawHit, sources: &mut Sources) -> Diagnostic {
    let (severity, w) = effective(rule, file);
    let params = params_for(rule, file);
    let f = |t: &Option<String>| fill(t.as_deref().unwrap_or(""), &hit.values, &params);
    let source = (hit.line > 0).then(|| {
        sources
            .get(file)
            .lines
            .get(hit.line - 1)
            .cloned()
            .unwrap_or_default()
    });
    Diagnostic {
        code: rule.code.clone(),
        severity,
        file: file.to_string(),
        line: hit.line,
        col: hit.col,
        length: hit.length,
        summary: f(&w.summary),
        why: f(&w.why),
        help: f(&w.help),
        label: w.label.as_ref().map(|_| f(&w.label)),
        source,
    }
}
