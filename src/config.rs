//! Loading and validating `necro.yaml` and its includes (spec: A rule,
//! Versioning). Every error is collected with its file, position and key
//! path; nothing runs until all of it loads.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};

use crate::command;
use crate::yaml::{self, Key, Node, Pos, Value};

/// The `necro.yaml` version this binary reads.
pub const SPEC_VERSION: i64 = 0;

/// One config problem, with where it lives.
#[derive(Debug, Clone)]
pub struct ConfigError {
    pub file: String,
    pub pos: Pos,
    pub path: String,
    pub message: String,
}

/// The bad and good snippets shown by `--explain`.
pub struct Examples {
    pub bad: Vec<String>,
    pub good: Vec<String>,
}

/// One house rule: its command plus the words that teach it.
pub struct Rule {
    pub name: String,
    /// The command as written; `None` for necro's own rules.
    pub command: Option<String>,
    pub message: String,
    pub why: String,
    pub fix: String,
    pub details: Option<String>,
    pub examples: Option<Examples>,
    /// The pack it came from (`rust` for `necro:rust`); `None` for the repo's own.
    pub pack: Option<String>,
    /// `files:`, for narrowed runs; `None` takes every file.
    pub files: Option<FileFilter>,
    /// `exclude:`: files outside the rule, in every run.
    pub exclude: Option<FileFilter>,
    /// Where it's defined: `necro.yaml`, an include file, or `necro:<pack>`.
    pub source: String,
    /// The keys `override:` in necro.yaml replaced, in spec order.
    pub overridden: Vec<&'static str>,
    /// `params:`, in the order written, with any override applied.
    pub params: Vec<Param>,
    /// `autofix:`, the command that fixes the rule's hits (spec: Fixes).
    pub autofix: Option<String>,
}

/// How a rule fixes its hits (spec: Fixes).
pub enum Fixer {
    /// Its command is a finder given a fix option; necro adds `--write`.
    Finder,
    /// Its `autofix:` command, given the files.
    Command,
}

/// The finders that can fix, and the words in a command that make it fixable.
const FIX_FINDERS: &[(&str, &[&str])] = &[
    ("grep", &["--replace"]),
    ("agent-files", crate::finders::AGENT_FILES_FIXABLE),
];

/// The `autofix:` word that becomes the files to fix.
pub const FILES_WORD: &str = "{files}";

/// One named value a rule's command and words use (spec: Parameters).
pub struct Param {
    pub name: String,
    /// The value's kind as `Node::kind` names it, or `a list`.
    pub kind: &'static str,
    /// The value as written; one item unless it's a list.
    pub values: Vec<String>,
    pub list: bool,
    /// Set by `override:` in necro.yaml.
    pub overridden: bool,
}

impl Param {
    fn joined(&self, sep: &str) -> String {
        self.values.join(sep)
    }
}

/// A `files:` or `exclude:` filter in prek's form (spec: Narrowed runs): a
/// regex searched in the path, or globs matched against the whole path.
pub enum FileFilter {
    Regex(fancy_regex::Regex, String),
    Glob(GlobSet, Vec<String>),
}

impl FileFilter {
    fn matches(&self, path: &str) -> bool {
        match self {
            // A regex too costly to finish counts as a match: running a rule
            // is the safe side.
            FileFilter::Regex(re, _) => re.is_match(path).unwrap_or(true),
            FileFilter::Glob(set, _) => set.is_match(path),
        }
    }
}

impl std::fmt::Display for FileFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FileFilter::Regex(_, source) => f.write_str(source),
            FileFilter::Glob(_, globs) => write!(f, "{{glob: [{}]}}", globs.join(", ")),
        }
    }
}

impl Rule {
    /// Whether a narrowed run over `given` runs this rule: always without
    /// `files:` or `exclude:`, else when some given file passes both.
    pub fn runs_on(&self, given: &[String]) -> bool {
        if self.files.is_none() && self.exclude.is_none() {
            return true;
        }
        given.iter().any(|path| {
            self.files.as_ref().is_none_or(|f| f.matches(path))
                && !self.exclude.as_ref().is_some_and(|f| f.matches(path))
        })
    }

    /// The command as words with its parameters in place: a word that is
    /// exactly `{name}` for a list becomes one word per item.
    pub fn words(&self) -> Vec<String> {
        let words = self
            .command
            .as_deref()
            .and_then(command::split)
            .unwrap_or_default();
        let mut out = Vec::new();
        for word in words {
            match self
                .params
                .iter()
                .find(|p| p.list && word == format!("{{{}}}", p.name))
            {
                Some(p) => out.extend(p.values.iter().cloned()),
                None => out.push(fill(&word, &self.params, ",")),
            }
        }
        out
    }

    /// How the rule fixes its hits, if it can.
    pub fn fixer(&self) -> Option<Fixer> {
        if self.autofix.is_some() {
            return Some(Fixer::Command);
        }
        let words = self.words();
        let finder = (words.first().map(String::as_str) == Some("necro"))
            .then(|| words.get(1))
            .flatten()?;
        let (_, options) = FIX_FINDERS.iter().find(|(f, _)| f == finder)?;
        let opts = words.iter().skip(2).take_while(|w| *w != "--");
        opts.into_iter()
            .any(|w| {
                options
                    .iter()
                    .any(|o| w == o || w.starts_with(&format!("{o}=")))
            })
            .then_some(Fixer::Finder)
    }

    /// The `autofix:` command as words: parameters in place, and the
    /// `{files}` word as one word per file.
    pub fn autofix_words(&self, files: &[String]) -> Vec<String> {
        let words = self
            .autofix
            .as_deref()
            .and_then(command::split)
            .unwrap_or_default();
        let mut out = Vec::new();
        for word in words {
            if word == FILES_WORD {
                out.extend(files.iter().cloned());
            } else {
                out.push(fill(&word, &self.params, ","));
            }
        }
        out
    }

    /// Put the parameters into the words (`message`, `why`, `fix`, `details`).
    fn fill_words(&mut self) {
        if self.params.is_empty() {
            return;
        }
        for text in [&mut self.message, &mut self.why, &mut self.fix] {
            *text = fill(text, &self.params, ", ");
        }
        if let Some(d) = &mut self.details {
            *d = fill(d, &self.params, ", ");
        }
    }

    /// Whether `path` is outside the rule (`exclude:`), so its hits there drop.
    pub fn excludes(&self, path: &str) -> bool {
        self.exclude.as_ref().is_some_and(|f| f.matches(path))
    }
}

/// `text` with each declared `{name}` replaced; other braces stay.
fn fill(text: &str, params: &[Param], sep: &str) -> String {
    let mut out = text.to_string();
    for p in params {
        out = out.replace(&format!("{{{}}}", p.name), &p.joined(sep));
    }
    out
}

/// The repo's rules, sorted by name.
pub struct Config {
    /// The repo's rules, sorted by name.
    pub rules: Vec<Rule>,
    /// Pack rules `disable:` turned off, sorted by name, for `--list`.
    pub disabled: Vec<Rule>,
    /// The snapshot's path, relative to the root; the default when absent.
    pub baseline: Option<String>,
    /// The top-level `exclude:`: files the whole run leaves out.
    pub exclude: Option<FileFilter>,
}

impl Config {
    /// The rule names, for unknown-rule errors.
    pub fn names(&self) -> Vec<&str> {
        self.rules.iter().map(|r| r.name.as_str()).collect()
    }

    /// Whether the top-level `exclude:` leaves `path` out of the run.
    pub fn excludes(&self, path: &str) -> bool {
        self.exclude.as_ref().is_some_and(|f| f.matches(path))
    }

    /// Where the snapshot lives: the `baseline:` pointer, else the default
    /// beside the root. A missing file means no baseline.
    pub fn baseline_path(&self, root: &Path) -> PathBuf {
        root.join(self.baseline.as_deref().unwrap_or(DEFAULT_BASELINE))
    }
}

struct Errors {
    list: Vec<ConfigError>,
}

impl Errors {
    fn at(&mut self, file: &str, pos: Pos, path: &str, message: impl Into<String>) {
        self.list.push(ConfigError {
            file: file.to_string(),
            pos,
            path: path.to_string(),
            message: message.into(),
        });
    }
}

/// A map node's entries, after checking every key is allowed.
fn entries<'n>(
    e: &mut Errors,
    file: &str,
    node: &'n Node,
    path: &str,
    allowed: &[&str],
) -> Option<BTreeMap<&'n str, (&'n Key, &'n Node)>> {
    let Value::Map(items) = &node.value else {
        e.at(
            file,
            node.pos,
            path,
            format!("expected a mapping, got {}", node.kind()),
        );
        return None;
    };
    let mut out = BTreeMap::new();
    for (key, value) in items {
        if allowed.contains(&key.name.as_str()) {
            out.insert(key.name.as_str(), (key, value));
        } else {
            let at = if path.is_empty() {
                key.name.clone()
            } else {
                format!("{path}.{}", key.name)
            };
            e.at(
                file,
                key.pos,
                &at,
                format!(
                    "unknown key `{}`; expected one of {}",
                    key.name,
                    allowed.join(", ")
                ),
            );
        }
    }
    Some(out)
}

fn string(e: &mut Errors, file: &str, node: &Node, path: &str) -> Option<String> {
    match node.as_str() {
        Some(s) => Some(s.to_string()),
        None => {
            e.at(
                file,
                node.pos,
                path,
                format!("expected a string, got {}", node.kind()),
            );
            None
        }
    }
}

/// A string, or a list of them.
fn snippets(e: &mut Errors, file: &str, node: &Node, path: &str) -> Option<Vec<String>> {
    match &node.value {
        Value::Seq(items) if !items.is_empty() => items
            .iter()
            .enumerate()
            .map(|(i, item)| string(e, file, item, &format!("{path}[{i}]")))
            .collect(),
        _ => string(e, file, node, path).map(|s| vec![s]),
    }
}

/// A text field: a string, or `{file: <path>}` relative to the root.
fn text(e: &mut Errors, root: &Path, file: &str, node: &Node, path: &str) -> Option<String> {
    if !matches!(node.value, Value::Map(_)) {
        return string(e, file, node, path);
    }
    let m = entries(e, file, node, path, &["file"])?;
    let Some((_, f)) = m.get("file") else {
        e.at(file, node.pos, path, "expected a string or {file: <path>}");
        return None;
    };
    let at = format!("{path}.file");
    let rel = string(e, file, f, &at)?;
    match std::fs::read_to_string(root.join(&rel)) {
        Ok(t) => Some(t),
        Err(err) => {
            e.at(file, f.pos, &at, format!("can't read {rel}: {err}"));
            None
        }
    }
}

/// Whether a rule name is lowercase words joined by hyphens.
pub fn valid_name(name: &str) -> bool {
    name.split('-').all(|w| {
        !w.is_empty()
            && w.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    })
}

/// The snapshot file when `baseline:` is absent: beside the root.
pub const DEFAULT_BASELINE: &str = "necro.baseline.yaml";

const TOP_KEYS: &[&str] = &[
    "version", "include", "disable", "override", "rules", "baseline", "exclude",
];
/// What `override:` may replace on a pack rule.
const OVERRIDE_KEYS: &[&str] = &["files", "exclude", "params"];

/// The prefix that names a pack in `include` (spec: Packs).
const PACK_PREFIX: &str = "necro:";
const INCLUDE_KEYS: &[&str] = &["rules"];
const RULE_KEYS: &[&str] = &[
    "command", "message", "why", "fix", "details", "examples", "files", "exclude", "params",
    "autofix",
];
const REQUIRED: &[&str] = &["command", "message", "why", "fix"];

fn read_yaml(e: &mut Errors, root: &Path, rel: &str) -> Option<Node> {
    let text = match std::fs::read_to_string(root.join(rel)) {
        Ok(t) => t,
        Err(err) => {
            e.at(
                rel,
                Pos { line: 1, col: 1 },
                "",
                format!("can't read: {err}"),
            );
            return None;
        }
    };
    match yaml::parse(&text) {
        Ok(node) if matches!(node.value, Value::Null) => Some(Node {
            value: Value::Map(vec![]),
            pos: node.pos,
        }),
        Ok(node) => Some(node),
        Err((pos, message)) => {
            e.at(rel, pos, "", message);
            None
        }
    }
}

/// Load and validate everything. Errors come back sorted by file and position.
pub fn load(root: &Path, files: &[String]) -> Result<Config, Vec<ConfigError>> {
    let mut e = Errors { list: Vec::new() };
    let top_file = "necro.yaml";
    let Some(top) = read_yaml(&mut e, root, top_file) else {
        return Err(e.list);
    };
    let Some(m) = entries(&mut e, top_file, &top, "", TOP_KEYS) else {
        return Err(e.list);
    };

    match m.get("version") {
        None => e.at(
            top_file,
            top.pos,
            "version",
            "missing required key `version`",
        ),
        Some((_, v)) => match v.as_number() {
            Some(n) if n == SPEC_VERSION as f64 => {}
            Some(n) if n.fract() == 0.0 && n > 0.0 => e.at(
                top_file,
                v.pos,
                "version",
                format!(
                    "this file is version {n}, and necro {} reads version {SPEC_VERSION}",
                    env!("CARGO_PKG_VERSION")
                ),
            ),
            _ => e.at(
                top_file,
                v.pos,
                "version",
                format!("version is a whole number; this necro reads {SPEC_VERSION}"),
            ),
        },
    }

    let baseline = m
        .get("baseline")
        .and_then(|(_, n)| string(&mut e, top_file, n, "baseline"));
    let exclude = m
        .get("exclude")
        .and_then(|(_, n)| file_filter(&mut e, top_file, n, "exclude"));

    // (file, pack, node): the repo's include files, then the packs.
    let mut sources: Vec<(String, Option<&str>, Node)> = Vec::new();
    if let Some((_, inc)) = m.get("include") {
        for rel in include_files(&mut e, top_file, inc, files) {
            if let Some(node) = read_yaml(&mut e, root, &rel) {
                sources.push((rel, None, node));
            }
        }
        for (name, text) in include_packs(&mut e, top_file, inc) {
            let file = format!("{PACK_PREFIX}{name}");
            match yaml::parse(text) {
                Ok(node) => sources.push((file, Some(name), node)),
                Err((pos, message)) => e.at(&file, pos, "", message),
            }
        }
    }

    let mut rules: Vec<Rule> = Vec::new();
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    let mut all: Vec<(&str, Option<&str>, Option<&Node>)> =
        vec![(top_file, None, m.get("rules").map(|(_, n)| *n))];
    for (rel, pack, node) in &sources {
        if let Some(inc) = entries(&mut e, rel, node, "", INCLUDE_KEYS) {
            all.push((rel, *pack, inc.get("rules").map(|(_, n)| *n)));
        }
    }
    for (file, pack, node) in all {
        let Some(node) = node else { continue };
        if matches!(node.value, Value::Null) {
            continue;
        }
        let Value::Map(items) = &node.value else {
            e.at(
                file,
                node.pos,
                "rules",
                format!("expected a mapping of rules, got {}", node.kind()),
            );
            continue;
        };
        for (key, value) in items {
            let path = format!("rules.{}", key.name);
            if !valid_name(&key.name) {
                e.at(
                    file,
                    key.pos,
                    &path,
                    format!(
                        "`{}` isn't a rule name: lowercase words joined by hyphens",
                        key.name
                    ),
                );
                continue;
            }
            let name = match pack {
                Some(p) => format!("{p}/{}", key.name),
                None => key.name.clone(),
            };
            if let Some(first) = seen.get(&name) {
                e.at(
                    file,
                    key.pos,
                    &path,
                    format!("rule `{name}` is already defined in {first}"),
                );
                continue;
            }
            seen.insert(name.clone(), file.to_string());
            if let Some(mut rule) = load_rule(&mut e, root, file, key, value) {
                rule.name = name;
                rule.pack = pack.map(String::from);
                rule.source = file.to_string();
                rules.push(rule);
            }
        }
    }

    let mut off_rules: Vec<Rule> = Vec::new();
    if let Some((_, node)) = m.get("disable") {
        let off = disabled(&mut e, top_file, node, &rules);
        (off_rules, rules) = rules.into_iter().partition(|r| off.contains(&r.name));
    }
    if let Some((_, node)) = m.get("override") {
        overrides(&mut e, top_file, node, &mut rules, &off_rules);
    }

    for rule in &mut rules {
        rule.fill_words();
    }

    if !e.list.is_empty() {
        let mut list = e.list;
        list.sort_by(|a, b| (&a.file, a.pos).cmp(&(&b.file, b.pos)));
        return Err(list);
    }
    rules.sort_by(|a, b| a.name.cmp(&b.name));
    off_rules.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(Config {
        rules,
        disabled: off_rules,
        baseline,
        exclude,
    })
}

/// The packs `include` names as `necro:<pack>`, each with its rules' YAML.
fn include_packs(e: &mut Errors, file: &str, node: &Node) -> Vec<(&'static str, &'static str)> {
    let Value::Seq(items) = &node.value else {
        return vec![];
    };
    let mut out: Vec<(&'static str, &'static str)> = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let Some(name) = item.as_str().and_then(|s| s.strip_prefix(PACK_PREFIX)) else {
            continue;
        };
        match crate::packs::find(name) {
            Some(pack) if !out.contains(&pack) => out.push(pack),
            Some(_) => {}
            None => e.at(
                file,
                item.pos,
                &format!("include[{i}]"),
                format!(
                    "necro has no pack `{name}`; packs: {}",
                    crate::packs::names().join(", ")
                ),
            ),
        }
    }
    out
}

/// The rule names `disable` turns off; each must be a rule of an included pack.
fn disabled(e: &mut Errors, file: &str, node: &Node, rules: &[Rule]) -> Vec<String> {
    let Value::Seq(items) = &node.value else {
        e.at(
            file,
            node.pos,
            "disable",
            format!("expected a list of pack rules, got {}", node.kind()),
        );
        return vec![];
    };
    let mut out = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let path = format!("disable[{i}]");
        let Some(name) = string(e, file, item, &path) else {
            continue;
        };
        if rules.iter().any(|r| r.name == name && r.pack.is_some()) {
            out.push(name);
        } else {
            e.at(
                file,
                item.pos,
                &path,
                format!("`{name}` isn't a rule of an included pack"),
            );
        }
    }
    out
}

/// Apply `override:`: each entry names a rule of an included pack that
/// `disable` leaves on, and replaces its `files` or `exclude`.
fn overrides(e: &mut Errors, file: &str, node: &Node, rules: &mut [Rule], off: &[Rule]) {
    let Value::Map(items) = &node.value else {
        e.at(
            file,
            node.pos,
            "override",
            format!("expected a mapping of pack rules, got {}", node.kind()),
        );
        return;
    };
    for (key, value) in items {
        let path = format!("override.{}", key.name);
        let name = key.name.as_str();
        let Some(rule) = rules
            .iter_mut()
            .find(|r| r.name == name && r.pack.is_some())
        else {
            let why = if off.iter().any(|r| r.name == name) {
                format!("`{name}` is turned off by disable; drop one of the two")
            } else {
                format!("`{name}` isn't a rule of an included pack")
            };
            e.at(file, key.pos, &path, why);
            continue;
        };
        let Some(m) = entries(e, file, value, &path, OVERRIDE_KEYS) else {
            continue;
        };
        if let Some((_, n)) = m.get("params") {
            override_params(e, file, n, &format!("{path}.params"), rule);
        }
        for k in &OVERRIDE_KEYS[..2] {
            let Some((_, n)) = m.get(k) else { continue };
            let Some(filter) = file_filter(e, file, n, &format!("{path}.{k}")) else {
                continue;
            };
            match *k {
                "files" => rule.files = Some(filter),
                _ => rule.exclude = Some(filter),
            }
            rule.overridden.push(k);
        }
    }
}

/// `override: {<rule>: {params: …}}`: each name must be declared, its value
/// of the default's kind.
fn override_params(e: &mut Errors, file: &str, node: &Node, path: &str, rule: &mut Rule) {
    let Some(given) = params(e, file, node, path) else {
        return;
    };
    let mut any = false;
    for (p, pos) in given {
        let at = format!("{path}.{}", p.name);
        let Some(declared) = rule.params.iter_mut().find(|d| d.name == p.name) else {
            let names: Vec<&str> = rule.params.iter().map(|d| d.name.as_str()).collect();
            let has = if names.is_empty() {
                "none".to_string()
            } else {
                names.join(", ")
            };
            e.at(
                file,
                pos.0,
                &at,
                format!(
                    "`{}` has no parameter `{}`; it has {has}",
                    rule.name, p.name
                ),
            );
            continue;
        };
        if declared.kind != p.kind {
            e.at(
                file,
                pos.1,
                &at,
                format!("`{}` is {}, not {}", p.name, declared.kind, p.kind),
            );
            continue;
        }
        declared.values = p.values;
        declared.overridden = true;
        any = true;
    }
    if any {
        rule.overridden.push("params");
    }
}

/// A `params:` mapping: each name, its value, and where the key and value are.
fn params(e: &mut Errors, file: &str, node: &Node, path: &str) -> Option<Vec<(Param, (Pos, Pos))>> {
    let Value::Map(items) = &node.value else {
        e.at(
            file,
            node.pos,
            path,
            format!("expected a mapping of parameters, got {}", node.kind()),
        );
        return None;
    };
    let mut out = Vec::new();
    for (key, value) in items {
        let at = format!("{path}.{}", key.name);
        if !valid_name(&key.name) {
            e.at(
                file,
                key.pos,
                &at,
                format!(
                    "`{}` isn't a parameter name: lowercase words joined by hyphens",
                    key.name
                ),
            );
            continue;
        }
        if key.name == "files" {
            e.at(
                file,
                key.pos,
                &at,
                "`files` is taken: {files} in autofix stands for the files to fix",
            );
            continue;
        }
        let scalar = |n: &Node| match &n.value {
            Value::Str(s) | Value::Plain(s) => Some(s.clone()),
            _ => None,
        };
        let (kind, values, list) = match &value.value {
            Value::Seq(items) => {
                let values: Option<Vec<String>> = items.iter().map(scalar).collect();
                match values {
                    Some(v) => ("a list", v, true),
                    None => {
                        e.at(
                            file,
                            value.pos,
                            &at,
                            "a list's items are strings, numbers or booleans",
                        );
                        continue;
                    }
                }
            }
            _ => match scalar(value) {
                Some(v) => (value.kind(), vec![v], false),
                None => {
                    e.at(
                        file,
                        value.pos,
                        &at,
                        format!(
                            "expected a string, number, boolean or a list of them, got {}",
                            value.kind()
                        ),
                    );
                    continue;
                }
            },
        };
        out.push((
            Param {
                name: key.name.clone(),
                kind,
                values,
                list,
                overridden: false,
            },
            (key.pos, value.pos),
        ));
    }
    Some(out)
}

/// The files the include globs match, sorted, without necro.yaml itself.
/// Pack entries (`necro:<pack>`) are left to `include_packs`.
fn include_files(e: &mut Errors, file: &str, node: &Node, files: &[String]) -> Vec<String> {
    let Value::Seq(items) = &node.value else {
        e.at(
            file,
            node.pos,
            "include",
            format!("expected a list of globs, got {}", node.kind()),
        );
        return vec![];
    };
    let mut set = GlobSetBuilder::new();
    for (i, item) in items.iter().enumerate() {
        let path = format!("include[{i}]");
        let Some(pattern) = string(e, file, item, &path) else {
            continue;
        };
        if pattern.starts_with(PACK_PREFIX) {
            continue;
        }
        match GlobBuilder::new(&pattern).literal_separator(true).build() {
            Ok(g) => {
                set.add(g);
            }
            Err(err) => e.at(file, item.pos, &path, format!("bad glob: {err}")),
        }
    }
    let Ok(set) = set.build() else { return vec![] };
    files
        .iter()
        .filter(|f| f.as_str() != "necro.yaml" && set.is_match(f.as_str()))
        .cloned()
        .collect()
}

fn load_rule(e: &mut Errors, root: &Path, file: &str, key: &Key, node: &Node) -> Option<Rule> {
    let path = format!("rules.{}", key.name);
    let m = entries(e, file, node, &path, RULE_KEYS)?;
    let before = e.list.len();
    for k in REQUIRED {
        if !m.contains_key(k) {
            e.at(
                file,
                key.pos,
                &format!("{path}.{k}"),
                format!("missing required key `{k}`"),
            );
        }
    }
    let at = |k: &str| format!("{path}.{k}");
    let command = m.get("command").and_then(|(_, n)| {
        let s = string(e, file, n, &at("command"))?;
        match command::split(&s) {
            Some(words) if !words.is_empty() => Some(s),
            Some(_) => {
                e.at(file, n.pos, &at("command"), "the command is empty");
                None
            }
            None => {
                e.at(file, n.pos, &at("command"), "an unclosed quote");
                None
            }
        }
    });
    let autofix = m.get("autofix").and_then(|(_, n)| {
        let s = string(e, file, n, &at("autofix"))?;
        match command::split(&s) {
            Some(words) if words.iter().any(|w| w == FILES_WORD) => Some(s),
            Some(_) => {
                e.at(
                    file,
                    n.pos,
                    &at("autofix"),
                    "autofix needs a {files} word: the files with hits to fix",
                );
                None
            }
            None => {
                e.at(file, n.pos, &at("autofix"), "an unclosed quote");
                None
            }
        }
    });
    let mut field = |k: &str| m.get(k).and_then(|(_, n)| text(e, root, file, n, &at(k)));
    let (message, why, fix, details) = (
        field("message"),
        field("why"),
        field("fix"),
        field("details"),
    );
    let examples = m.get("examples").and_then(|(k, n)| {
        let ex = entries(e, file, n, &at("examples"), &["bad", "good"])?;
        let mut side = |name: &str| match ex.get(name) {
            Some((_, n)) => snippets(e, file, n, &format!("{path}.examples.{name}")),
            None => {
                e.at(
                    file,
                    k.pos,
                    &format!("{path}.examples.{name}"),
                    format!("examples need both bad and good; `{name}` is missing"),
                );
                None
            }
        };
        let (bad, good) = (side("bad"), side("good"));
        Some(Examples {
            bad: bad?,
            good: good?,
        })
    });
    let files = m
        .get("files")
        .and_then(|(_, n)| file_filter(e, file, n, &at("files")));
    let exclude = m
        .get("exclude")
        .and_then(|(_, n)| file_filter(e, file, n, &at("exclude")));
    let params = m
        .get("params")
        .and_then(|(_, n)| params(e, file, n, &at("params")))
        .map(|ps| ps.into_iter().map(|(p, _)| p).collect())
        .unwrap_or_default();
    if e.list.len() > before {
        return None;
    }
    Some(Rule {
        name: key.name.clone(),
        command,
        message: message?,
        why: why?,
        fix: fix?,
        details,
        examples,
        pack: None,
        files,
        exclude,
        source: file.to_string(),
        overridden: Vec::new(),
        params,
        autofix,
    })
}

/// `files:` or `exclude:`: a regex string, or `{glob: <glob or list>}` with
/// globset's defaults, so `*` crosses `/` as in prek.
fn file_filter(e: &mut Errors, file: &str, node: &Node, path: &str) -> Option<FileFilter> {
    if let Some(source) = node.as_str() {
        return match fancy_regex::Regex::new(source) {
            Ok(re) => Some(FileFilter::Regex(re, source.to_string())),
            Err(err) => {
                e.at(
                    file,
                    node.pos,
                    path,
                    format!("bad regex: {}", crate::finders::regex_error(&err)),
                );
                None
            }
        };
    }
    if !matches!(node.value, Value::Map(_)) {
        e.at(
            file,
            node.pos,
            path,
            format!(
                "expected a regex or {{glob: <glob or list>}}, got {}",
                node.kind()
            ),
        );
        return None;
    }
    let before = e.list.len();
    let m = entries(e, file, node, path, &["glob"])?;
    let Some((_, value)) = m.get("glob") else {
        // An unknown key was already reported.
        if e.list.len() == before {
            e.at(file, node.pos, path, "expected {glob: <glob or list>}");
        }
        return None;
    };
    let at = format!("{path}.glob");
    let globs: Vec<(String, Pos, String)> = match &value.value {
        Value::Seq(items) if !items.is_empty() => items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let p = format!("{at}[{i}]");
                string(e, file, item, &p).map(|g| (g, item.pos, p))
            })
            .collect::<Option<_>>()?,
        _ => vec![(string(e, file, value, &at)?, value.pos, at.clone())],
    };
    let mut set = GlobSetBuilder::new();
    let mut ok = true;
    for (glob, pos, p) in &globs {
        match GlobBuilder::new(glob).build() {
            Ok(g) => {
                set.add(g);
            }
            Err(err) => {
                e.at(file, *pos, p, format!("bad glob: {}", err.kind()));
                ok = false;
            }
        }
    }
    let set = set.build().ok().filter(|_| ok)?;
    Some(FileFilter::Glob(
        set,
        globs.into_iter().map(|(g, _, _)| g).collect(),
    ))
}

/// necro's own rules for allows (spec: Allows); their message names the allow's rule.
pub fn builtins() -> [Rule; 2] {
    let rule = |name: &str, message: &str, why: &str, fix: &str| Rule {
        name: name.to_string(),
        command: None,
        message: message.to_string(),
        why: why.to_string(),
        fix: fix.to_string(),
        details: None,
        examples: None,
        pack: None,
        files: None,
        exclude: None,
        source: String::new(),
        overridden: Vec::new(),
        params: Vec::new(),
        autofix: None,
    };
    [
        rule(
            "necro/allow-reason",
            "An allow without a reason",
            "An allow without a reason can't be reviewed or revisited later.",
            "Write the reason after the rule: necro-allow <rule>: <why this case is fair>.",
        ),
        rule(
            "necro/unused-allow",
            "An allow that suppresses nothing",
            "A stale allow hides the next real hit on its line, and a mistyped rule allows nothing.",
            "Delete the allow, or fix the rule it names.",
        ),
    ]
}
