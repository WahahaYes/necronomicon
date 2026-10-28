//! Loading and validating `necro.yaml` and `necro/<group>.yaml` (spec: Master,
//! Groups and rules, Checks, Scope and parameters). Every error is collected
//! with its file, position and key path; nothing runs until all of it loads.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use globset::{Glob, GlobBuilder, GlobSet, GlobSetBuilder};
use regex::Regex;

use crate::lang::{self, Lang};
use crate::yaml::{self, Node, Pos, Value};

pub const SPEC_VERSION: i64 = 0;

#[derive(Debug, Clone)]
pub struct ConfigError {
    pub file: String,
    pub pos: Pos,
    pub path: String,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Error,
    Experimental,
}

impl Severity {
    pub fn name(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Experimental => "experimental",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParamType {
    Str,
    Number,
    Bool,
    StrList,
    StrMap,
}

impl ParamType {
    pub fn parse(s: &str) -> Option<ParamType> {
        Some(match s {
            "string" => ParamType::Str,
            "number" => ParamType::Number,
            "boolean" => ParamType::Bool,
            "string[]" => ParamType::StrList,
            "map<string>" => ParamType::StrMap,
            _ => return None,
        })
    }

    fn name(self) -> &'static str {
        match self {
            ParamType::Str => "a string",
            ParamType::Number => "a number",
            ParamType::Bool => "a boolean",
            ParamType::StrList => "a list of strings",
            ParamType::StrMap => "a mapping of strings",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum ParamValue {
    Str(String),
    Number(f64),
    Bool(bool),
    StrList(Vec<String>),
    StrMap(BTreeMap<String, String>),
}

#[derive(Clone, Debug)]
pub struct ParamDecl {
    pub ty: ParamType,
    /// None: the rule must set it.
    pub default: Option<ParamValue>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Scope {
    File,
    Project,
}

#[derive(Clone, Debug)]
pub struct CheckDecl {
    pub scope: Scope,
    pub params: BTreeMap<String, ParamDecl>,
    pub values: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Engine {
    CommentBlocks,
    Require,
    DocAbove,
    AllowReason,
    UnusedAllow,
}

impl Engine {
    fn parse(name: &str) -> Option<Engine> {
        Some(match name {
            "comment-blocks" => Engine::CommentBlocks,
            "require" => Engine::Require,
            "doc-above" => Engine::DocAbove,
            "allow-reason" => Engine::AllowReason,
            "unused-allow" => Engine::UnusedAllow,
            _ => return None,
        })
    }

    fn decl(self) -> CheckDecl {
        let p = |ty, default| ParamDecl { ty, default };
        let (params, values): (Vec<(&str, ParamDecl)>, &[&str]) = match self {
            Engine::CommentBlocks => (
                vec![
                    (
                        "header",
                        p(ParamType::Number, Some(ParamValue::Number(10.0))),
                    ),
                    ("block", p(ParamType::Number, Some(ParamValue::Number(6.0)))),
                    (
                        "markers",
                        p(ParamType::StrList, Some(ParamValue::StrList(vec![]))),
                    ),
                ],
                &["lines", "limit"],
            ),
            Engine::Require => (
                vec![
                    ("patterns", p(ParamType::StrList, None)),
                    (
                        "at",
                        p(ParamType::Str, Some(ParamValue::Str("anywhere".into()))),
                    ),
                    (
                        "in",
                        p(ParamType::Str, Some(ParamValue::Str("code".into()))),
                    ),
                ],
                &["pattern"],
            ),
            Engine::DocAbove => (
                vec![
                    ("target", p(ParamType::Str, None)),
                    (
                        "skip",
                        p(ParamType::Str, Some(ParamValue::Str(String::new()))),
                    ),
                    (
                        "markers",
                        p(ParamType::StrList, Some(ParamValue::StrList(vec![]))),
                    ),
                ],
                &[],
            ),
            Engine::AllowReason | Engine::UnusedAllow => (vec![], &["code"]),
        };
        CheckDecl {
            scope: Scope::File,
            params: params
                .into_iter()
                .map(|(k, d)| (k.to_string(), d))
                .collect(),
            values: values.iter().map(|v| v.to_string()).collect(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum Check {
    Regex { re: Regex, raw_line: bool },
    Engine(Engine),
    Script { group: String, name: String },
}

#[derive(Clone, Debug, Default)]
pub struct Wording {
    pub summary: Option<String>,
    pub why: Option<String>,
    pub help: Option<String>,
    pub label: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Where {
    pub files: GlobSet,
    pub ignores: GlobSet,
    pub files_src: Vec<String>,
    pub params: BTreeMap<String, ParamValue>,
    pub severity: Option<Severity>,
    pub wording: Wording,
    /// Keys this entry sets (`params.x` per param), for overlap errors.
    pub keys: Vec<String>,
    pub pos: Pos,
}

#[derive(Clone, Debug)]
pub struct Rule {
    pub code: String,
    pub group: String,
    pub file: String,
    pub wording: Wording,
    pub explain: String,
    pub check: Check,
    pub decl: CheckDecl,
    pub files: GlobSet,
    pub ignores: GlobSet,
    pub params: BTreeMap<String, ParamValue>,
    pub severity: Severity,
    pub wheres: Vec<Where>,
    pub pos: Pos,
}

#[derive(Clone, Debug)]
pub struct Group {
    pub name: String,
    pub guide: String,
    pub command: Option<Vec<String>>,
    pub timeout: f64,
}

pub struct Config {
    pub files: GlobSet,
    pub ignores: GlobSet,
    pub groups: Vec<Group>,
    pub rules: Vec<Rule>,
    pub langs: BTreeMap<String, Lang>,
}

impl Config {
    pub fn lang(&self, path: &str) -> Option<&Lang> {
        self.langs.get(&lang::extension(path))
    }
}

/// Declarations of a group's scripted checks, from its `describe`.
pub type Describe<'a> = dyn FnMut(&Group) -> Result<BTreeMap<String, CheckDecl>, String> + 'a;

/// A group whose `describe` failed: rules using it can't be validated.
pub struct BrokenGroup {
    pub group: String,
    pub message: String,
    /// An `error` rule uses one of its checks, so the run fails.
    pub fatal: bool,
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

fn join(path: &str, key: &str) -> String {
    if path.is_empty() {
        key.to_string()
    } else {
        format!("{path}.{key}")
    }
}

/// A map node's entries, after checking every key is allowed.
fn entries<'n>(
    e: &mut Errors,
    file: &str,
    node: &'n Node,
    path: &str,
    allowed: &[&str],
) -> Option<BTreeMap<&'n str, &'n Node>> {
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
            out.insert(key.name.as_str(), value);
        } else {
            e.at(
                file,
                key.pos,
                &join(path, &key.name),
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

fn string_list(e: &mut Errors, file: &str, node: &Node, path: &str) -> Option<Vec<(String, Pos)>> {
    let Value::Seq(items) = &node.value else {
        e.at(
            file,
            node.pos,
            path,
            format!("expected a list of strings, got {}", node.kind()),
        );
        return None;
    };
    let mut out = Vec::new();
    for (i, item) in items.iter().enumerate() {
        out.push((string(e, file, item, &format!("{path}[{i}]"))?, item.pos));
    }
    Some(out)
}

pub fn glob(pattern: &str) -> Result<Glob, globset::Error> {
    GlobBuilder::new(pattern).literal_separator(true).build()
}

fn globs(e: &mut Errors, file: &str, node: &Node, path: &str) -> Option<(GlobSet, Vec<String>)> {
    let items = string_list(e, file, node, path)?;
    let mut set = GlobSetBuilder::new();
    let mut ok = true;
    for (i, (pattern, pos)) in items.iter().enumerate() {
        match glob(pattern) {
            Ok(g) => {
                set.add(g);
            }
            Err(err) => {
                e.at(
                    file,
                    *pos,
                    &format!("{path}[{i}]"),
                    format!("invalid glob: {}", err.kind()),
                );
                ok = false;
            }
        }
    }
    let set = set.build().ok()?;
    ok.then(|| (set, items.into_iter().map(|(p, _)| p).collect()))
}

fn globset_of(patterns: &[&str]) -> GlobSet {
    let mut set = GlobSetBuilder::new();
    for p in patterns {
        if let Ok(g) = glob(p) {
            set.add(g);
        }
    }
    set.build().unwrap_or_else(|_| GlobSet::empty())
}

fn severity(e: &mut Errors, file: &str, node: &Node, path: &str) -> Option<Severity> {
    match node.as_str() {
        Some("error") => Some(Severity::Error),
        Some("experimental") => Some(Severity::Experimental),
        _ => {
            e.at(
                file,
                node.pos,
                path,
                "severity is `error` or `experimental`",
            );
            None
        }
    }
}

fn param_value(
    e: &mut Errors,
    file: &str,
    node: &Node,
    path: &str,
    ty: ParamType,
) -> Option<ParamValue> {
    let value = match ty {
        ParamType::Str => node.as_str().map(|s| ParamValue::Str(s.into())),
        ParamType::Number => node.as_number().map(ParamValue::Number),
        ParamType::Bool => node.as_bool().map(ParamValue::Bool),
        ParamType::StrList => match &node.value {
            Value::Seq(items) => items
                .iter()
                .map(|i| i.as_str().map(String::from))
                .collect::<Option<Vec<_>>>()
                .map(ParamValue::StrList),
            _ => None,
        },
        ParamType::StrMap => match &node.value {
            Value::Map(items) => items
                .iter()
                .map(|(k, v)| v.as_str().map(|s| (k.name.clone(), s.to_string())))
                .collect::<Option<BTreeMap<_, _>>>()
                .map(ParamValue::StrMap),
            _ => None,
        },
    };
    if value.is_none() {
        e.at(
            file,
            node.pos,
            path,
            format!("expected {}, got {}", ty.name(), node.kind()),
        );
    }
    value
}

/// Values for params given at one level, checked against the rule's declaration.
fn params(
    e: &mut Errors,
    file: &str,
    node: &Node,
    path: &str,
    decl: &CheckDecl,
) -> BTreeMap<String, ParamValue> {
    let mut out = BTreeMap::new();
    let Value::Map(items) = &node.value else {
        if !matches!(node.value, Value::Null) {
            e.at(
                file,
                node.pos,
                path,
                format!("expected a mapping, got {}", node.kind()),
            );
        }
        return out;
    };
    for (key, value) in items {
        let p = join(path, &key.name);
        match decl.params.get(&key.name) {
            None => e.at(
                file,
                key.pos,
                &p,
                format!("the check declares no parameter `{}`", key.name),
            ),
            Some(d) => {
                if let Some(v) = param_value(e, file, value, &p, d.ty) {
                    out.insert(key.name.clone(), v);
                }
            }
        }
    }
    out
}

/// `{{name}}` placeholders in a template.
pub fn placeholders(template: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        let Some(end) = rest[start..].find("}}") else {
            break;
        };
        out.push(rest[start + 2..start + end].trim().to_string());
        rest = &rest[start + end + 2..];
    }
    out
}

fn wording_key(
    e: &mut Errors,
    file: &str,
    node: &Node,
    path: &str,
    decl: &CheckDecl,
) -> Option<String> {
    let text = string(e, file, node, path)?;
    for name in placeholders(&text) {
        let ok = decl.values.contains(&name)
            || decl
                .params
                .get(&name)
                .is_some_and(|d| d.ty != ParamType::StrMap);
        if !ok {
            let why = if decl.params.contains_key(&name) {
                format!("`{{{{{name}}}}}` is a map parameter, which wording can't use")
            } else {
                format!(
                    "`{{{{{name}}}}}` is neither a value the check fills nor one of its parameters"
                )
            };
            e.at(file, node.pos, path, why);
        }
    }
    Some(text)
}

fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.split('-').all(|w| {
            !w.is_empty()
                && w.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        })
}

const MASTER_KEYS: &[&str] = &[
    "version",
    "files",
    "ignores",
    "groups",
    "defaults",
    "languages",
];
const DEFAULT_KEYS: &[&str] = &["files", "ignores", "params", "severity"];
const GROUP_KEYS: &[&str] = &[
    "guide", "files", "ignores", "params", "severity", "command", "timeout", "rules",
];
const RULE_KEYS: &[&str] = &[
    "summary", "why", "help", "explain", "check", "label", "files", "ignores", "params", "where",
    "severity",
];
const WHERE_KEYS: &[&str] = &[
    "files", "ignores", "params", "severity", "summary", "why", "help", "label",
];

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
        Ok(node) => Some(node),
        Err((pos, message)) => {
            e.at(rel, pos, "", message);
            None
        }
    }
}

/// Scope keys at one level: (files, ignores), each with its source patterns.
#[derive(Clone, Default)]
struct ScopeKeys {
    files: Option<(GlobSet, Vec<String>)>,
    ignores: Option<(GlobSet, Vec<String>)>,
}

struct GroupSpec<'n> {
    group: Group,
    file: String,
    scope: ScopeKeys,
    params: Option<&'n Node>,
    severity: Option<Severity>,
    rules: Vec<(&'n yaml::Key, &'n Node)>,
    rules_pos: Pos,
}

/// The engine's own rules for allows, required in the `necro` group.
const NECRO_GROUP: &str = "necro";

pub struct Loaded {
    pub config: Config,
    pub broken: Vec<BrokenGroup>,
}

/// Load and validate everything. Errors come back sorted by file and position.
pub fn load(root: &Path, describe: &mut Describe) -> Result<Loaded, Vec<ConfigError>> {
    let mut e = Errors { list: Vec::new() };
    let master_file = "necro.yaml";
    let Some(master) = read_yaml(&mut e, root, master_file) else {
        return Err(e.list);
    };
    let empty = Node {
        value: Value::Map(vec![]),
        pos: master.pos,
    };
    let master = if matches!(master.value, Value::Null) {
        &empty
    } else {
        &master
    };
    let Some(m) = entries(&mut e, master_file, master, "", MASTER_KEYS) else {
        return Err(e.list);
    };

    match m.get("version") {
        None => e.at(master_file, master.pos, "version", "missing required key `version`"),
        Some(v) => match v.as_number() {
            Some(n) if n == SPEC_VERSION as f64 => {}
            Some(n) if n > SPEC_VERSION as f64 => e.at(
                master_file,
                v.pos,
                "version",
                format!(
                    "this file is version {n}, and necro {} reads version {SPEC_VERSION}; raise the hook's rev",
                    env!("CARGO_PKG_VERSION")
                ),
            ),
            _ => e.at(master_file, v.pos, "version", format!("version is a number; this necro reads {SPEC_VERSION}")),
        },
    }
    let all = || Some((globset_of(&["**/*"]), vec!["**/*".to_string()]));
    let files = m
        .get("files")
        .map_or_else(all, |n| globs(&mut e, master_file, n, "files"));
    let ignores = m.get("ignores").map_or_else(
        || Some((GlobSet::empty(), vec![])),
        |n| globs(&mut e, master_file, n, "ignores"),
    );

    let mut defaults = ScopeKeys::default();
    let mut default_params: Option<&Node> = None;
    let mut default_severity = None;
    if let Some(d) = m.get("defaults")
        && let Some(d) = entries(&mut e, master_file, d, "defaults", DEFAULT_KEYS)
    {
        defaults.files = d
            .get("files")
            .and_then(|n| globs(&mut e, master_file, n, "defaults.files"));
        defaults.ignores = d
            .get("ignores")
            .and_then(|n| globs(&mut e, master_file, n, "defaults.ignores"));
        default_params = d.get("params").copied();
        default_severity = d
            .get("severity")
            .and_then(|n| severity(&mut e, master_file, n, "defaults.severity"));
    }

    let mut langs = lang::builtin();
    if let Some(l) = m.get("languages") {
        load_languages(&mut e, master_file, l, &mut langs);
    }

    // Group files on disk against the list.
    let mut on_disk = BTreeSet::new();
    if let Ok(dir) = std::fs::read_dir(root.join("necro")) {
        for entry in dir.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(stem) = name.strip_suffix(".yaml")
                && entry.path().is_file()
            {
                on_disk.insert(stem.to_string());
            }
        }
    }
    let mut listed: Vec<(String, Pos)> = Vec::new();
    match m.get("groups") {
        None => e.at(
            master_file,
            master.pos,
            "groups",
            "missing required key `groups`",
        ),
        Some(g) => {
            if let Some(items) = string_list(&mut e, master_file, g, "groups") {
                for (i, (name, pos)) in items.iter().enumerate() {
                    if !on_disk.contains(name) {
                        e.at(
                            master_file,
                            *pos,
                            &format!("groups[{i}]"),
                            format!("group `{name}` has no file necro/{name}.yaml"),
                        );
                    }
                }
                listed = items;
                let names: BTreeSet<&String> = listed.iter().map(|(n, _)| n).collect();
                for stem in &on_disk {
                    if !names.contains(stem) {
                        e.at(
                            master_file,
                            g.pos,
                            "groups",
                            format!("necro/{stem}.yaml isn't listed; list it, or delete the file"),
                        );
                    }
                }
                if !names.contains(&NECRO_GROUP.to_string()) {
                    e.at(
                        master_file,
                        g.pos,
                        "groups",
                        "the `necro` group is required: it words the engine's allow errors",
                    );
                }
            }
        }
    }

    // Parse each listed group's own keys.
    let mut nodes: Vec<(String, Node)> = Vec::new();
    for (name, _) in &listed {
        let rel = format!("necro/{name}.yaml");
        if on_disk.contains(name)
            && let Some(node) = read_yaml(&mut e, root, &rel)
        {
            nodes.push((name.clone(), node));
        }
    }
    let mut specs: Vec<GroupSpec> = Vec::new();
    for (name, node) in &nodes {
        let file = format!("necro/{name}.yaml");
        if let Some(spec) = group_spec(&mut e, &file, name, node) {
            specs.push(spec);
        }
    }

    // Describe the groups whose checks are used.
    let mut decls: BTreeMap<String, BTreeMap<String, CheckDecl>> = BTreeMap::new();
    let mut broken = Vec::new();
    let mut used = BTreeSet::new();
    for spec in &specs {
        for (_, rule) in &spec.rules {
            if let Value::Map(items) = &rule.value
                && let Some((_, check)) = items.iter().find(|(k, _)| k.name == "check")
                && let Some(s) = check.as_str()
                && let Some((g, _)) = s.split_once('.')
                && g != "engine"
            {
                used.insert(g.to_string());
            }
        }
    }
    for spec in &specs {
        if spec.group.command.is_some() && used.contains(&spec.group.name) {
            match describe(&spec.group) {
                Ok(d) => {
                    decls.insert(spec.group.name.clone(), d);
                }
                Err(message) => broken.push(BrokenGroup {
                    group: spec.group.name.clone(),
                    message,
                    fatal: specs
                        .iter()
                        .any(|s| uses_as_error(s, &spec.group.name, default_severity)),
                }),
            }
        }
    }
    let commands: BTreeSet<String> = specs
        .iter()
        .filter(|s| s.group.command.is_some())
        .map(|s| s.group.name.clone())
        .collect();

    let mut rules = Vec::new();
    let mut declared_anywhere: BTreeSet<String> = BTreeSet::new();
    for spec in &specs {
        let mut declared_here: BTreeSet<String> = BTreeSet::new();
        let mut engines_here = BTreeSet::new();
        for (key, node) in &spec.rules {
            let ctx = RuleCtx {
                spec,
                defaults: &defaults,
                default_params,
                default_severity,
                decls: &decls,
                commands: &commands,
                broken: &broken,
            };
            if let Some(rule) = load_rule(&mut e, &ctx, key, node) {
                declared_here.extend(rule.decl.params.keys().cloned());
                if let Check::Engine(engine) = rule.check {
                    engines_here.insert(format!("{engine:?}"));
                }
                rules.push(rule);
            }
        }
        if let Some(p) = spec.params
            && let Value::Map(items) = &p.value
        {
            for (key, _) in items {
                if !declared_here.contains(&key.name) {
                    e.at(
                        &spec.file,
                        key.pos,
                        &format!("params.{}", key.name),
                        format!("no rule in this group has a check declaring `{}`", key.name),
                    );
                }
            }
        }
        if spec.group.name == NECRO_GROUP {
            for (engine, name) in [
                ("AllowReason", "engine.allow-reason"),
                ("UnusedAllow", "engine.unused-allow"),
            ] {
                if !engines_here.contains(engine) {
                    e.at(
                        &spec.file,
                        spec.rules_pos,
                        "rules",
                        format!("the necro group needs a rule using {name}"),
                    );
                }
            }
        }
        declared_anywhere.extend(declared_here);
    }
    if let Some(p) = default_params
        && let Value::Map(items) = &p.value
    {
        for (key, _) in items {
            if !declared_anywhere.contains(&key.name) {
                e.at(
                    master_file,
                    key.pos,
                    &format!("defaults.params.{}", key.name),
                    format!("no rule has a check declaring `{}`", key.name),
                );
            }
        }
    }

    if !e.list.is_empty() {
        let mut list = e.list;
        list.sort_by(|a, b| (&a.file, a.pos).cmp(&(&b.file, b.pos)));
        return Err(list);
    }
    let (files, ignores) = (files.map(|f| f.0), ignores.map(|i| i.0));
    Ok(Loaded {
        config: Config {
            files: files.unwrap_or_else(GlobSet::empty),
            ignores: ignores.unwrap_or_else(GlobSet::empty),
            groups: specs.into_iter().map(|s| s.group).collect(),
            rules,
            langs,
        },
        broken,
    })
}

/// Whether a rule with error severity, here or in a `where`, uses one of `group`'s checks.
fn uses_as_error(spec: &GroupSpec, group: &str, default: Option<Severity>) -> bool {
    let is_error = |n: Option<&Node>| n.and_then(Node::as_str) == Some("error");
    spec.rules.iter().any(|(_, rule)| {
        let Value::Map(items) = &rule.value else {
            return false;
        };
        let get = |k: &str| items.iter().find(|(key, _)| key.name == k).map(|(_, v)| v);
        let uses = get("check")
            .and_then(Node::as_str)
            .and_then(|c| c.split_once('.'))
            .is_some_and(|(g, _)| g == group);
        let own = get("severity").and_then(Node::as_str);
        let base = match own {
            Some(s) => s == "error",
            None => spec.severity.or(default).unwrap_or(Severity::Error) == Severity::Error,
        };
        let wheres = match get("where").map(|w| &w.value) {
            Some(Value::Seq(entries)) => entries.iter().any(|e| match &e.value {
                Value::Map(kv) => is_error(
                    kv.iter()
                        .find(|(k, _)| k.name == "severity")
                        .map(|(_, v)| v),
                ),
                _ => false,
            }),
            _ => false,
        };
        uses && (base || wheres)
    })
}

fn load_languages(e: &mut Errors, file: &str, node: &Node, langs: &mut BTreeMap<String, Lang>) {
    let Value::Map(items) = &node.value else {
        e.at(
            file,
            node.pos,
            "languages",
            format!("expected a mapping, got {}", node.kind()),
        );
        return;
    };
    for (key, value) in items {
        let path = format!("languages.{}", key.name);
        if !key.name.starts_with('.') {
            e.at(
                file,
                key.pos,
                &path,
                "a language is keyed by its extension, with the dot",
            );
            continue;
        }
        let Some(m) = entries(e, file, value, &path, &["like", "line", "block", "strings"]) else {
            continue;
        };
        if let Some(like) = m.get("like") {
            let Some(name) = string(e, file, like, &join(&path, "like")) else {
                continue;
            };
            match langs.get(&name) {
                Some(l) if m.len() == 1 => {
                    let l = l.clone();
                    langs.insert(key.name.clone(), l);
                }
                Some(_) => e.at(
                    file,
                    like.pos,
                    &join(&path, "like"),
                    "`like` copies a language whole; give no other keys",
                ),
                None => e.at(
                    file,
                    like.pos,
                    &join(&path, "like"),
                    format!("no language `{name}` to copy"),
                ),
            }
            continue;
        }
        let list = |e: &mut Errors, k: &str| -> Vec<String> {
            m.get(k)
                .and_then(|n| string_list(e, file, n, &join(&path, k)))
                .map(|v| v.into_iter().map(|(s, _)| s).collect())
                .unwrap_or_default()
        };
        let line = list(e, "line");
        let strings = list(e, "strings");
        let mut block = Vec::new();
        if let Some(b) = m.get("block") {
            if let Value::Seq(pairs) = &b.value {
                for (i, pair) in pairs.iter().enumerate() {
                    match string_list(e, file, pair, &format!("{path}.block[{i}]")).as_deref() {
                        Some([(o, _), (c, _)]) => block.push((o.clone(), c.clone())),
                        Some(_) => e.at(
                            file,
                            pair.pos,
                            &format!("{path}.block[{i}]"),
                            "a block comment is [opener, closer]",
                        ),
                        None => {}
                    }
                }
            } else {
                e.at(
                    file,
                    b.pos,
                    &join(&path, "block"),
                    "expected a list of [opener, closer] pairs",
                );
            }
        }
        langs.insert(key.name.clone(), lang::custom(line, block, strings));
    }
}

fn group_spec<'n>(e: &mut Errors, file: &str, name: &str, node: &'n Node) -> Option<GroupSpec<'n>> {
    let empty_pos = node.pos;
    let g = match &node.value {
        Value::Null => BTreeMap::new(),
        _ => entries(e, file, node, "", GROUP_KEYS)?,
    };
    let guide = match g.get("guide") {
        Some(n) => string(e, file, n, "guide").unwrap_or_default(),
        None => {
            e.at(file, empty_pos, "guide", "missing required key `guide`");
            String::new()
        }
    };
    let scope = ScopeKeys {
        files: g.get("files").and_then(|n| globs(e, file, n, "files")),
        ignores: g.get("ignores").and_then(|n| globs(e, file, n, "ignores")),
    };
    let severity = g
        .get("severity")
        .and_then(|n| severity(e, file, n, "severity"));
    let command = g.get("command").and_then(|n| match &n.value {
        Value::Seq(_) => {
            string_list(e, file, n, "command").map(|v| v.into_iter().map(|(s, _)| s).collect())
        }
        _ => match n.as_str().map(split_command) {
            Some(Some(argv)) if !argv.is_empty() => Some(argv),
            _ => {
                e.at(
                    file,
                    n.pos,
                    "command",
                    "expected an argv list or a command string",
                );
                None
            }
        },
    });
    let timeout = match g.get("timeout") {
        None => 60.0,
        Some(n) => match n.as_number() {
            Some(t) if t > 0.0 => t,
            _ => {
                e.at(
                    file,
                    n.pos,
                    "timeout",
                    "timeout is a positive number of seconds",
                );
                60.0
            }
        },
    };
    let (rules, rules_pos) = match g.get("rules") {
        None => {
            e.at(file, empty_pos, "rules", "missing required key `rules`");
            (vec![], empty_pos)
        }
        Some(r) => match &r.value {
            Value::Map(items) => (items.iter().map(|(k, v)| (k, v)).collect(), r.pos),
            _ => {
                e.at(
                    file,
                    r.pos,
                    "rules",
                    format!("expected a mapping of rules, got {}", r.kind()),
                );
                (vec![], r.pos)
            }
        },
    };
    // A block mapping's position is its first key; point at `rules:` itself.
    let rules_pos = match &node.value {
        Value::Map(items) => items
            .iter()
            .find(|(k, _)| k.name == "rules")
            .map_or(rules_pos, |(k, _)| k.pos),
        _ => rules_pos,
    };
    Some(GroupSpec {
        group: Group {
            name: name.to_string(),
            guide,
            command,
            timeout,
        },
        file: file.to_string(),
        scope,
        params: g.get("params").copied(),
        severity,
        rules,
        rules_pos,
    })
}

/// Split a command string as a POSIX shell would (quotes and backslashes; no expansion).
pub fn split_command(s: &str) -> Option<Vec<String>> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_word = false;
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        match c {
            '\'' => {
                in_word = true;
                loop {
                    match chars.next()? {
                        '\'' => break,
                        c => cur.push(c),
                    }
                }
            }
            '"' => {
                in_word = true;
                loop {
                    match chars.next()? {
                        '"' => break,
                        '\\' => cur.push(chars.next()?),
                        c => cur.push(c),
                    }
                }
            }
            '\\' => {
                in_word = true;
                cur.push(chars.next()?);
            }
            c if c.is_whitespace() => {
                if in_word {
                    out.push(std::mem::take(&mut cur));
                    in_word = false;
                }
            }
            c => {
                in_word = true;
                cur.push(c);
            }
        }
    }
    if in_word {
        out.push(cur);
    }
    Some(out)
}

struct RuleCtx<'a, 'n> {
    spec: &'a GroupSpec<'n>,
    defaults: &'a ScopeKeys,
    default_params: Option<&'a Node>,
    default_severity: Option<Severity>,
    decls: &'a BTreeMap<String, BTreeMap<String, CheckDecl>>,
    commands: &'a BTreeSet<String>,
    broken: &'a [BrokenGroup],
}

fn resolve_check(
    e: &mut Errors,
    ctx: &RuleCtx,
    node: &Node,
    path: &str,
) -> Option<(Check, CheckDecl)> {
    let file = &ctx.spec.file;
    if let Some(s) = node.as_str() {
        let Some((group, name)) = s.split_once('.') else {
            e.at(
                file,
                node.pos,
                path,
                format!("`{s}` isn't a check: use engine.<name>, <group>.<name> or {{regex: …}}"),
            );
            return None;
        };
        if group == "engine" {
            let Some(engine) = Engine::parse(name) else {
                e.at(file, node.pos, path, format!("no built-in check `{s}`; built-ins: engine.comment-blocks, engine.require, engine.doc-above, engine.allow-reason, engine.unused-allow"));
                return None;
            };
            return Some((Check::Engine(engine), engine.decl()));
        }
        if !ctx.commands.contains(group) {
            e.at(
                file,
                node.pos,
                path,
                format!("group `{group}` has no `command`, so it has no check `{name}`"),
            );
            return None;
        }
        if ctx.broken.iter().any(|b| b.group == group) {
            return None;
        }
        let decl = ctx.decls.get(group).and_then(|d| d.get(name));
        let Some(decl) = decl else {
            let known: Vec<&String> = ctx
                .decls
                .get(group)
                .map(|d| d.keys().collect())
                .unwrap_or_default();
            e.at(
                file,
                node.pos,
                path,
                format!(
                    "group `{group}` describes no check `{name}`; it has: {}",
                    known
                        .iter()
                        .map(|s| s.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            );
            return None;
        };
        return Some((
            Check::Script {
                group: group.into(),
                name: name.into(),
            },
            decl.clone(),
        ));
    }
    let Value::Map(items) = &node.value else {
        e.at(
            file,
            node.pos,
            path,
            format!("expected a check name or {{regex: …}}, got {}", node.kind()),
        );
        return None;
    };
    if let Some((key, _)) = items.iter().find(|(k, _)| k.name == "ast-grep") {
        e.at(
            file,
            key.pos,
            &join(path, "ast-grep"),
            "ast-grep checks arrive in a later 0.x",
        );
        return None;
    }
    let m = entries(e, file, node, path, &["regex", "in"])?;
    let raw_line = match m.get("in") {
        None => false,
        Some(n) => match n.as_str() {
            Some("code") => false,
            Some("line") => true,
            _ => {
                e.at(file, n.pos, &join(path, "in"), "`in` is `code` or `line`");
                false
            }
        },
    };
    let Some(pattern) = m.get("regex") else {
        e.at(file, node.pos, path, "a check mapping needs `regex`");
        return None;
    };
    let p = join(path, "regex");
    let text = string(e, file, pattern, &p)?;
    match Regex::new(&text) {
        Ok(re) => {
            let values = re.capture_names().flatten().map(String::from).collect();
            Some((
                Check::Regex { re, raw_line },
                CheckDecl {
                    scope: Scope::File,
                    params: BTreeMap::new(),
                    values,
                },
            ))
        }
        Err(err) => {
            e.at(
                file,
                pattern.pos,
                &p,
                format!(
                    "invalid regex: {}",
                    err.to_string().lines().last().unwrap_or("")
                ),
            );
            None
        }
    }
}

/// Engine params that must be valid regexes or one of a few words.
fn check_engine_params(
    e: &mut Errors,
    file: &str,
    path: &str,
    pos: Pos,
    engine: Engine,
    params: &BTreeMap<String, ParamValue>,
    decl: &mut CheckDecl,
) {
    let regex_ok = |e: &mut Errors, key: &str, s: &str| match Regex::new(s) {
        Ok(re) => Some(re),
        Err(err) => {
            e.at(
                file,
                pos,
                &format!("{path}.{key}"),
                format!(
                    "invalid regex: {}",
                    err.to_string().lines().last().unwrap_or("")
                ),
            );
            None
        }
    };
    match engine {
        Engine::Require => {
            if let Some(ParamValue::StrList(list)) = params.get("patterns") {
                for p in list {
                    regex_ok(e, "patterns", p);
                }
            }
            if let Some(ParamValue::Str(at)) = params.get("at")
                && at != "start"
                && at != "anywhere"
            {
                e.at(
                    file,
                    pos,
                    &format!("{path}.at"),
                    "`at` is `start` or `anywhere`",
                );
            }
            if let Some(ParamValue::Str(i)) = params.get("in")
                && i != "code"
                && i != "line"
            {
                e.at(file, pos, &format!("{path}.in"), "`in` is `code` or `line`");
            }
        }
        Engine::DocAbove => {
            if let Some(ParamValue::Str(t)) = params.get("target")
                && let Some(re) = regex_ok(e, "target", t)
            {
                decl.values = re.capture_names().flatten().map(String::from).collect();
            }
            if let Some(ParamValue::Str(s)) = params.get("skip")
                && !s.is_empty()
            {
                regex_ok(e, "skip", s);
            }
        }
        _ => {}
    }
}

fn load_rule(e: &mut Errors, ctx: &RuleCtx, key: &yaml::Key, node: &Node) -> Option<Rule> {
    let spec = ctx.spec;
    let file = spec.file.as_str();
    let path = format!("rules.{}", key.name);
    if !valid_name(&key.name) {
        e.at(
            file,
            key.pos,
            &path,
            "a rule name is lowercase words joined by hyphens",
        );
        return None;
    }
    let r = entries(e, file, node, &path, RULE_KEYS)?;
    for required in ["summary", "why", "help", "explain", "check"] {
        if !r.contains_key(required) {
            e.at(
                file,
                key.pos,
                &join(&path, required),
                format!("missing required key `{required}`"),
            );
        }
    }
    let (check, mut decl) = resolve_check(e, ctx, r.get("check")?, &join(&path, "check"))?;

    // Params: rule, then group, then master defaults, then the declared default.
    let mut given = BTreeMap::new();
    let levels = [
        (
            ctx.default_params,
            "necro.yaml",
            "defaults.params".to_string(),
        ),
        (spec.params, file, "params".to_string()),
        (r.get("params").copied(), file, join(&path, "params")),
    ];
    for (level, (n, f, p)) in levels.iter().enumerate() {
        let Some(n) = n else { continue };
        if level < 2 {
            // Group and master defaults set only what this rule's check declares.
            if let Value::Map(items) = &n.value {
                for (k, v) in items {
                    if let Some(d) = decl.params.get(&k.name)
                        && let Some(value) = param_value(e, f, v, &join(p, &k.name), d.ty)
                    {
                        given.insert(k.name.clone(), value);
                    }
                }
            }
        } else {
            given.extend(params(e, f, n, p, &decl));
        }
    }
    // A param given with a bad value is already an error; don't also call it missing.
    let mentioned: BTreeSet<&str> = levels
        .iter()
        .filter_map(|(n, _, _)| match n.map(|n| &n.value) {
            Some(Value::Map(items)) => Some(
                items
                    .iter()
                    .map(|(k, _)| k.name.as_str())
                    .collect::<Vec<_>>(),
            ),
            _ => None,
        })
        .flatten()
        .collect();
    let mut resolved = BTreeMap::new();
    for (name, d) in &decl.params {
        match given.get(name).or(d.default.as_ref()) {
            Some(v) => {
                resolved.insert(name.clone(), v.clone());
            }
            None if mentioned.contains(name.as_str()) => {}
            None => e.at(
                file,
                key.pos,
                &format!("{path}.params.{name}"),
                format!("the check needs parameter `{name}`"),
            ),
        }
    }
    if let Check::Engine(engine) = check {
        let pos = r.get("params").map_or(key.pos, |n| n.pos);
        check_engine_params(
            e,
            file,
            &join(&path, "params"),
            pos,
            engine,
            &resolved,
            &mut decl,
        );
    }

    let text = |e: &mut Errors, k: &str| {
        r.get(k)
            .and_then(|n| wording_key(e, file, n, &join(&path, k), &decl))
    };
    let wording = Wording {
        summary: text(e, "summary"),
        why: text(e, "why"),
        help: text(e, "help"),
        label: text(e, "label"),
    };
    let explain = r
        .get("explain")
        .and_then(|n| string(e, file, n, &join(&path, "explain")))
        .unwrap_or_default();

    let pick = |own: Option<(GlobSet, Vec<String>)>,
                group: &Option<(GlobSet, Vec<String>)>,
                master: &Option<(GlobSet, Vec<String>)>| {
        own.or_else(|| group.clone()).or_else(|| master.clone())
    };
    let own_files = r
        .get("files")
        .and_then(|n| globs(e, file, n, &join(&path, "files")));
    let own_ignores = r
        .get("ignores")
        .and_then(|n| globs(e, file, n, &join(&path, "ignores")));
    let files = pick(own_files, &spec.scope.files, &ctx.defaults.files)
        .map_or_else(|| globset_of(&["**/*"]), |f| f.0);
    let ignores = pick(own_ignores, &spec.scope.ignores, &ctx.defaults.ignores)
        .map_or_else(GlobSet::empty, |f| f.0);
    let severity = r
        .get("severity")
        .and_then(|n| severity(e, file, n, &join(&path, "severity")))
        .or(spec.severity)
        .or(ctx.default_severity)
        .unwrap_or(Severity::Error);

    let mut wheres = Vec::new();
    if let Some(w) = r.get("where") {
        let wpath = join(&path, "where");
        match &w.value {
            Value::Seq(items) => {
                for (i, item) in items.iter().enumerate() {
                    if let Some(entry) = load_where(e, file, item, &format!("{wpath}[{i}]"), &decl)
                    {
                        wheres.push(entry);
                    }
                }
            }
            _ => e.at(
                file,
                w.pos,
                &wpath,
                format!("expected a list of entries, got {}", w.kind()),
            ),
        }
    }
    Some(Rule {
        code: format!("{}/{}", spec.group.name, key.name),
        group: spec.group.name.clone(),
        file: file.to_string(),
        wording,
        explain,
        check,
        decl,
        files,
        ignores,
        params: resolved,
        severity,
        wheres,
        pos: key.pos,
    })
}

fn load_where(
    e: &mut Errors,
    file: &str,
    node: &Node,
    path: &str,
    decl: &CheckDecl,
) -> Option<Where> {
    let w = entries(e, file, node, path, WHERE_KEYS)?;
    let Some(files_node) = w.get("files") else {
        e.at(
            file,
            node.pos,
            &join(path, "files"),
            "a where entry needs `files`",
        );
        return None;
    };
    let (files, files_src) = globs(e, file, files_node, &join(path, "files"))?;
    let ignores = w
        .get("ignores")
        .and_then(|n| globs(e, file, n, &join(path, "ignores")))
        .map_or_else(GlobSet::empty, |g| g.0);
    let mut keys = Vec::new();
    let params = match w.get("params") {
        Some(n) => {
            if decl.scope == Scope::Project {
                e.at(
                    file,
                    n.pos,
                    &join(path, "params"),
                    "a project check runs once over every file, so its params can't vary by where",
                );
            }
            params(e, file, n, &join(path, "params"), decl)
        }
        None => BTreeMap::new(),
    };
    keys.extend(params.keys().map(|k| format!("params.{k}")));
    let severity = w
        .get("severity")
        .and_then(|n| severity(e, file, n, &join(path, "severity")));
    if severity.is_some() {
        keys.push("severity".into());
    }
    let mut text = |k: &str| {
        let v = w
            .get(k)
            .and_then(|n| wording_key(e, file, n, &join(path, k), decl));
        if v.is_some() {
            keys.push(k.to_string());
        }
        v
    };
    let wording = Wording {
        summary: text("summary"),
        why: text("why"),
        help: text("help"),
        label: text("label"),
    };
    Some(Where {
        files,
        ignores,
        files_src,
        params,
        severity,
        wording,
        keys,
        pos: node.pos,
    })
}

/// Two `where` entries that both match a file and set one key: found from the run's files.
pub fn where_overlaps(config: &Config, files: &[String]) -> Vec<ConfigError> {
    let mut out = Vec::new();
    for rule in &config.rules {
        let mut seen = BTreeSet::new();
        for f in files {
            if !rule.files.is_match(f) || rule.ignores.is_match(f) {
                continue;
            }
            let hits: Vec<usize> = (0..rule.wheres.len())
                .filter(|&i| rule.wheres[i].matches(f))
                .collect();
            for (a, &i) in hits.iter().enumerate() {
                for &j in &hits[a + 1..] {
                    for key in &rule.wheres[j].keys {
                        if rule.wheres[i].keys.contains(key) && seen.insert((i, j, key.clone())) {
                            let name = rule.code.split_once('/').map_or("", |(_, n)| n);
                            out.push(ConfigError {
                                file: rule.file.clone(),
                                pos: rule.wheres[j].pos,
                                path: format!("rules.{name}.where[{j}].{key}"),
                                message: format!(
                                    "where[{i}] and where[{j}] both set `{key}` for {f}"
                                ),
                            });
                        }
                    }
                }
            }
        }
    }
    out
}

impl Where {
    pub fn matches(&self, file: &str) -> bool {
        self.files.is_match(file) && !self.ignores.is_match(file)
    }
}
