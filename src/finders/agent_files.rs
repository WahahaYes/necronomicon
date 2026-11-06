//! `necro agent-files` (spec: Finders): the files coding agents read, kept
//! in the shape every harness finds: AGENTS.md with a CLAUDE.md beside it,
//! skills in `.agents/skills` linked from `.claude/skills`. It checks the
//! whole working folder, since the files are judged in pairs.

use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use regex::Regex;

use super::grep::read_list;

const USAGE: &str = "necro agent-files [--write [--only-at <file>]] \
     (claude-md|claude-md-alone|skill-link|skill-misplaced|skill-frontmatter|\
     other-instructions|rule-reference|stale-skill)";

/// What a CLAUDE.md holds when AGENTS.md has the content.
const IMPORT: &str = "@AGENTS.md";

/// Instruction files other harnesses read, whose content belongs in AGENTS.md.
const OTHER: &[&str] = &[
    ".cursorrules",
    ".windsurfrules",
    ".clinerules",
    ".github/copilot-instructions.md",
    "GEMINI.md",
];

/// One tracked path: whether git records it as a symlink.
struct Entry {
    link: bool,
}

/// The checks `--write` fixes.
pub const FIXABLE: &[&str] = &["claude-md", "skill-link", "stale-skill"];

/// The `agent-files` finder.
pub fn run(args: &[String]) -> Result<Vec<String>, String> {
    let mut write = false;
    let mut only_at: Option<String> = None;
    let mut check: Option<&str> = None;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--write" => write = true,
            "--only-at" => {
                only_at = Some(
                    it.next()
                        .ok_or(format!("--only-at needs a value: {USAGE}"))?
                        .clone(),
                )
            }
            a if a.starts_with("--only-at=") => only_at = Some(a["--only-at=".len()..].to_string()),
            // A narrowed run's list: these checks look at pairs, so the whole folder.
            "--files-from" => {
                it.next();
            }
            f if f.starts_with('-') => return Err(format!("unknown option {f}: {USAGE}")),
            c if check.is_none() => check = Some(c),
            other => return Err(format!("unexpected argument {other}: {USAGE}")),
        }
    }
    let check = check.ok_or(format!("name a check: {USAGE}"))?;
    if (write || only_at.is_some()) && !FIXABLE.contains(&check) {
        return Err(format!(
            "--write goes with claude-md, skill-link and stale-skill: {USAGE}"
        ));
    }
    let index = index()?;
    let hits = match check {
        "claude-md" => claude_md(&index),
        "claude-md-alone" => claude_md_alone(&index),
        "skill-link" => skill_link(&index),
        "skill-misplaced" => skill_misplaced(&index),
        "skill-frontmatter" => skill_frontmatter(&index),
        "other-instructions" => other_instructions(&index),
        "rule-reference" => rule_reference(&index)?,
        "stale-skill" => stale_skill(&index),
        other => return Err(format!("unknown check {other}: {USAGE}")),
    };
    if !write {
        return Ok(hits);
    }
    let only: Option<HashSet<String>> = only_at.as_deref().map(read_list).transpose()?.map(|l| {
        l.into_iter()
            .map(|a| a.split(':').next().unwrap_or_default().to_string())
            .collect()
    });
    let mut fixed = Vec::new();
    for hit in hits {
        let path = hit.split(':').next().unwrap_or_default().to_string();
        if only.as_ref().is_some_and(|o| !o.contains(&path)) {
            continue;
        }
        if fix(check, &path)? {
            fixed.push(format!("{path}: fixed"));
        }
    }
    Ok(fixed)
}

/// Every path under the working folder that git tracks, or would track (an
/// untracked file not ignored, such as one `--fix` just created), that still
/// exists: git's index says which tracked ones are symlinks, the disk the rest.
fn index() -> Result<BTreeMap<String, Entry>, String> {
    let list = |args: &[&str]| -> Result<String, String> {
        let out = crate::files::git()
            .args(args)
            .output()
            .map_err(|e| format!("can't run git: {e}"))?;
        if !out.status.success() {
            return Err("git ls-files failed".into());
        }
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    };
    let on_disk = |p: &str| std::fs::symlink_metadata(p).ok();
    let mut out = BTreeMap::new();
    for rec in list(&["ls-files", "-s", "-z"])?.split('\0') {
        if let Some((meta, path)) = rec.split_once('\t')
            && on_disk(path).is_some()
        {
            out.insert(
                path.to_string(),
                Entry {
                    link: meta.starts_with("120000"),
                },
            );
        }
    }
    for path in list(&["ls-files", "-z", "-o", "--exclude-standard"])?.split('\0') {
        if let Some(meta) = on_disk(path).filter(|_| !path.is_empty()) {
            out.insert(
                path.to_string(),
                Entry {
                    link: meta.file_type().is_symlink(),
                },
            );
        }
    }
    Ok(out)
}

/// A tracked symlink's target: the link on disk, or, where the checkout
/// wrote links as plain files (Windows without symlinks), the file's text.
fn target(path: &str) -> String {
    std::fs::read_link(path)
        .map(|t| t.to_string_lossy().into_owned())
        .or_else(|_| std::fs::read_to_string(path))
        .unwrap_or_default()
        .trim()
        .trim_end_matches('/')
        .to_string()
}

/// The folder part of a path, with its trailing slash ("" at the top).
fn dir_of(path: &str) -> &str {
    path.rfind('/').map_or("", |i| &path[..=i])
}

/// The file name part of a path.
fn base(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// `claude-md`: each AGENTS.md has a CLAUDE.md beside it holding only `@AGENTS.md` (or linking to it).
fn claude_md(index: &BTreeMap<String, Entry>) -> Vec<String> {
    let mut hits = Vec::new();
    for path in index.keys().filter(|p| base(p) == "AGENTS.md") {
        let claude = format!("{}CLAUDE.md", dir_of(path));
        match index.get(&claude) {
            None => hits.push(format!("{path}: no CLAUDE.md beside it")),
            Some(e) if e.link => {
                if target(&claude) != "AGENTS.md" {
                    hits.push(format!(
                        "{claude}: links to {}, not AGENTS.md",
                        target(&claude)
                    ));
                }
            }
            Some(_) => {
                let text = std::fs::read_to_string(&claude).unwrap_or_default();
                if text.trim() != IMPORT {
                    hits.push(format!("{claude}: holds more than {IMPORT}"));
                }
            }
        }
    }
    hits
}

/// `claude-md-alone`: a CLAUDE.md with no AGENTS.md beside it.
fn claude_md_alone(index: &BTreeMap<String, Entry>) -> Vec<String> {
    index
        .keys()
        .filter(|p| {
            base(p) == "CLAUDE.md" && !index.contains_key(&format!("{}AGENTS.md", dir_of(p)))
        })
        .map(|p| format!("{p}: no AGENTS.md beside it"))
        .collect()
}

/// Skills in `.agents/skills`: (prefix before `.agents/`, name) for each tracked `SKILL.md`.
fn agent_skills(index: &BTreeMap<String, Entry>) -> Vec<(String, String)> {
    let Ok(re) = Regex::new(r"^(.*?)\.agents/skills/([^/]+)/SKILL\.md$") else {
        return vec![];
    };
    index
        .keys()
        .filter_map(|p| re.captures(p).map(|c| (c[1].to_string(), c[2].to_string())))
        .collect()
}

/// `skill-link`: each skill in `.agents/skills` has a `.claude/skills/<name>` link to it.
fn skill_link(index: &BTreeMap<String, Entry>) -> Vec<String> {
    let mut hits = Vec::new();
    for (prefix, name) in agent_skills(index) {
        let skill = format!("{prefix}.agents/skills/{name}/SKILL.md");
        let link = format!("{prefix}.claude/skills/{name}");
        let want = format!("../../.agents/skills/{name}");
        match index.get(&link) {
            Some(e) if e.link && target(&link) == want => {}
            Some(e) if e.link => hits.push(format!(
                "{skill}: {link} links to {}, not {want}",
                target(&link)
            )),
            // A real folder there is skill-misplaced's to report.
            _ if index.keys().any(|p| p.starts_with(&format!("{link}/"))) => {}
            _ => hits.push(format!("{skill}: no {link} link")),
        }
    }
    hits
}

/// `skill-misplaced`: a real skill folder in `.claude/skills`, or a link there to no skill.
fn skill_misplaced(index: &BTreeMap<String, Entry>) -> Vec<String> {
    let Ok(re) = Regex::new(r"^(.*?)\.claude/skills/([^/]+)(/.*)?$") else {
        return vec![];
    };
    let mut seen = HashSet::new();
    let mut hits = Vec::new();
    for (path, entry) in index {
        let Some(c) = re.captures(path) else { continue };
        let (prefix, name) = (&c[1], &c[2]);
        if !seen.insert(format!("{prefix}{name}")) {
            continue;
        }
        if c.get(3).is_some() {
            let folder = format!("{prefix}.claude/skills/{name}");
            let file = [format!("{folder}/SKILL.md"), path.clone()]
                .into_iter()
                .find(|f| index.contains_key(f))
                .unwrap_or_default();
            hits.push(format!(
                "{file}: a skill folder in .claude/skills; it belongs in .agents/skills"
            ));
        } else if entry.link {
            let skill = format!("{prefix}.agents/skills/{name}/SKILL.md");
            if !index.contains_key(&skill) {
                hits.push(format!(
                    "{path}: links to {}, which isn't a skill in .agents/skills",
                    target(path)
                ));
            }
        }
    }
    hits
}

/// The frontmatter lines of a SKILL.md, with their 1-based numbers, or `None` without one.
fn frontmatter(path: &str) -> Option<Vec<(usize, String)>> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut lines = text.lines().enumerate();
    if lines.next()?.1.trim() != "---" {
        return None;
    }
    Some(
        lines
            .take_while(|(_, l)| l.trim() != "---")
            .map(|(i, l)| (i + 1, l.to_string()))
            .collect(),
    )
}

/// A frontmatter key's value at the top level, unquoted, with its line.
fn key<'a>(fm: &'a [(usize, String)], name: &str) -> Option<(usize, &'a str)> {
    fm.iter().find_map(|(n, l)| {
        let v = l.strip_prefix(name)?.strip_prefix(':')?;
        Some((*n, v.trim().trim_matches(|c| c == '"' || c == '\'')))
    })
}

/// `skill-frontmatter`: each SKILL.md opens with frontmatter naming its folder and saying what it's for.
fn skill_frontmatter(index: &BTreeMap<String, Entry>) -> Vec<String> {
    let mut hits = Vec::new();
    for path in index.keys().filter(|p| base(p) == "SKILL.md") {
        let folder = base(dir_of(path).trim_end_matches('/'));
        let Some(fm) = frontmatter(path) else {
            hits.push(format!("{path}:1:1: no --- frontmatter"));
            continue;
        };
        match key(&fm, "name") {
            None => hits.push(format!("{path}:1:1: no name")),
            Some((n, v)) if v != folder => hits.push(format!("{path}:{n}:1: name {v} isn't the folder's, {folder}")),
            Some((n, v)) if !crate::config::valid_name(v) || v.len() > 64 => {
                hits.push(format!("{path}:{n}:1: name {v} isn't lowercase words joined by hyphens, up to 64 characters"))
            }
            Some(_) => {}
        }
        match key(&fm, "description") {
            Some((_, v)) if !v.is_empty() && v.len() <= 1024 => {}
            Some((n, v)) if !v.is_empty() => hits.push(format!(
                "{path}:{n}:1: description over 1024 characters ({})",
                v.len()
            )),
            _ => hits.push(format!("{path}:1:1: no description")),
        }
    }
    hits
}

/// `other-instructions`: another harness's instruction file with content that doesn't point to AGENTS.md.
fn other_instructions(index: &BTreeMap<String, Entry>) -> Vec<String> {
    index
        .keys()
        .filter(|p| {
            OTHER
                .iter()
                .any(|o| *p == o || p.ends_with(&format!("/{o}")))
        })
        .filter(|p| {
            let text = std::fs::read_to_string(p).unwrap_or_default();
            !text.trim().is_empty() && !text.contains("AGENTS.md")
        })
        .map(|p| format!("{p}: instructions outside AGENTS.md"))
        .collect()
}

/// `rule-reference`: a rule named in AGENTS.md, CLAUDE.md or a SKILL.md that the config here doesn't have.
fn rule_reference(index: &BTreeMap<String, Entry>) -> Result<Vec<String>, String> {
    let cwd = std::env::current_dir().map_err(|e| format!("can't read the current folder: {e}"))?;
    let root = crate::files::find_root(&cwd).ok_or("no necro.yaml here or above")?;
    let config = crate::config::load(&root, &crate::files::list(&root))
        .map_err(|_| "necro.yaml doesn't load; run necro for the errors".to_string())?;
    let known: HashSet<String> = config
        .rules
        .iter()
        .map(|r| r.name.clone())
        .chain(crate::config::builtins().iter().map(|r| r.name.clone()))
        .collect();
    let packs = crate::packs::names();
    let Ok(re) = Regex::new(
        r"necro\s+(?:--explain|--rule)\s+([a-z0-9][a-z0-9/-]*)|`([a-z0-9][a-z0-9-]*/[a-z0-9][a-z0-9-]*)`",
    ) else {
        return Ok(vec![]);
    };
    let mut hits = Vec::new();
    for path in index
        .keys()
        .filter(|p| ["AGENTS.md", "CLAUDE.md", "SKILL.md"].contains(&base(p)))
    {
        let text = std::fs::read_to_string(path).unwrap_or_default();
        for (n, line) in text.lines().enumerate() {
            for c in re.captures_iter(line) {
                let (m, explicit) = match (c.get(1), c.get(2)) {
                    (Some(m), _) => (m, true),
                    (None, Some(m)) => (m, false),
                    _ => continue,
                };
                let name = m.as_str();
                // A backticked `a/b` names a rule only when `a` is a pack.
                let pack_rule = name
                    .split_once('/')
                    .is_some_and(|(p, _)| packs.contains(&p));
                if known.contains(name) || (!explicit && !pack_rule) {
                    continue;
                }
                let col = line[..m.start()].chars().count() + 1;
                hits.push(format!(
                    "{path}:{}:{col}-{}: {name}",
                    n + 1,
                    col + name.chars().count() - 1
                ));
            }
        }
    }
    Ok(hits)
}

/// The version an installed necro skill was written by, from `metadata: necro-version:`.
fn stamped_version(path: &str) -> Option<(usize, String)> {
    let fm = frontmatter(path)?;
    let start = fm.iter().position(|(_, l)| l.trim_end() == "metadata:")?;
    fm[start + 1..]
        .iter()
        .take_while(|(_, l)| l.starts_with(' '))
        .find_map(|(n, l)| {
            let v = l.trim().strip_prefix("necro-version:")?;
            Some((
                *n,
                v.trim().trim_matches(|c| c == '"' || c == '\'').to_string(),
            ))
        })
}

/// `stale-skill`: a skill `necro skills install` wrote for another necro version.
fn stale_skill(index: &BTreeMap<String, Entry>) -> Vec<String> {
    let current = env!("CARGO_PKG_VERSION");
    index
        .keys()
        .filter(|p| base(p) == "SKILL.md" && !index.get(*p).is_some_and(|e| e.link))
        .filter_map(|p| {
            let (n, v) = stamped_version(p)?;
            (v != current).then(|| format!("{p}:{n}:1: from necro {v}; this is necro {current}"))
        })
        .collect()
}

/// Fix one hit's file; whether anything changed.
fn fix(check: &str, path: &str) -> Result<bool, String> {
    match check {
        "claude-md" if base(path) == "AGENTS.md" => {
            let claude = format!("{}CLAUDE.md", dir_of(path));
            if Path::new(&claude).exists() {
                return Ok(false);
            }
            std::fs::write(&claude, format!("{IMPORT}\n"))
                .map_err(|e| format!("can't write {claude}: {e}"))?;
            Ok(true)
        }
        "skill-link" => {
            let Some((prefix, rest)) = path.split_once(".agents/skills/") else {
                return Ok(false);
            };
            let name = rest.split('/').next().unwrap_or_default();
            let folder = format!("{prefix}.claude/skills");
            let link = format!("{folder}/{name}");
            let meta = std::fs::symlink_metadata(&link);
            if meta.as_ref().is_ok_and(|m| !m.file_type().is_symlink()) {
                return Ok(false);
            }
            if meta.is_ok() {
                std::fs::remove_file(&link).map_err(|e| format!("can't replace {link}: {e}"))?;
            }
            std::fs::create_dir_all(&folder).map_err(|e| format!("can't create {folder}: {e}"))?;
            crate::files::symlink(&format!("../../.agents/skills/{name}"), Path::new(&link))?;
            Ok(true)
        }
        "stale-skill" if path.contains(".agents/skills/") => {
            let name = base(dir_of(path).trim_end_matches('/'));
            let Some(text) = crate::skills::stamped(name) else {
                return Ok(false);
            };
            std::fs::write(path, text).map_err(|e| format!("can't write {path}: {e}"))?;
            Ok(true)
        }
        _ => Ok(false),
    }
}
