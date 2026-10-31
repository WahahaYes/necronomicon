//! Loading and validating `necro.yaml` and its includes (spec: A rule,
//! Versioning). Every error is collected with its file, position and key
//! path; nothing runs until all of it loads.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use globset::{GlobBuilder, GlobSetBuilder};

use crate::command;
use crate::yaml::{self, Key, Node, Pos, Value};

pub const SPEC_VERSION: i64 = 0;

#[derive(Debug, Clone)]
pub struct ConfigError {
    pub file: String,
    pub pos: Pos,
    pub path: String,
    pub message: String,
}

pub struct Examples {
    pub bad: Vec<String>,
    pub good: Vec<String>,
}

pub struct Rule {
    pub name: String,
    /// The command as written; `None` for necro's own rules.
    pub command: Option<String>,
    pub message: String,
    pub why: String,
    pub fix: String,
    pub details: Option<String>,
    pub examples: Option<Examples>,
}

pub struct Config {
    /// The repo's rules, sorted by name.
    pub rules: Vec<Rule>,
    /// The snapshot's path, relative to the root; the default when absent.
    pub baseline: Option<String>,
}

impl Config {
    pub fn names(&self) -> Vec<&str> {
        self.rules.iter().map(|r| r.name.as_str()).collect()
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

pub fn valid_name(name: &str) -> bool {
    name.split('-').all(|w| {
        !w.is_empty()
            && w.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    })
}

/// The snapshot file when `baseline:` is absent: beside the root.
pub const DEFAULT_BASELINE: &str = "necro.baseline.yaml";

const TOP_KEYS: &[&str] = &["version", "include", "rules", "baseline"];
const INCLUDE_KEYS: &[&str] = &["rules"];
const RULE_KEYS: &[&str] = &["command", "message", "why", "fix", "details", "examples"];
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

    let mut sources: Vec<(String, Node)> = Vec::new();
    if let Some((_, inc)) = m.get("include") {
        for rel in include_files(&mut e, top_file, inc, files) {
            if let Some(node) = read_yaml(&mut e, root, &rel) {
                sources.push((rel, node));
            }
        }
    }

    let mut rules: Vec<Rule> = Vec::new();
    let mut seen: BTreeMap<String, String> = BTreeMap::new();
    let mut all: Vec<(&str, Option<&Node>)> = vec![(top_file, m.get("rules").map(|(_, n)| *n))];
    for (rel, node) in &sources {
        if let Some(inc) = entries(&mut e, rel, node, "", INCLUDE_KEYS) {
            all.push((rel, inc.get("rules").map(|(_, n)| *n)));
        }
    }
    for (file, node) in all {
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
            if let Some(first) = seen.get(&key.name) {
                e.at(
                    file,
                    key.pos,
                    &path,
                    format!("rule `{}` is already defined in {first}", key.name),
                );
                continue;
            }
            seen.insert(key.name.clone(), file.to_string());
            if let Some(rule) = load_rule(&mut e, root, file, key, value) {
                rules.push(rule);
            }
        }
    }

    if !e.list.is_empty() {
        let mut list = e.list;
        list.sort_by(|a, b| (&a.file, a.pos).cmp(&(&b.file, b.pos)));
        return Err(list);
    }
    rules.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(Config { rules, baseline })
}

/// The files the include globs match, sorted, without necro.yaml itself.
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
    })
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
