//! Finders (spec: Finders): ordinary commands necro bundles for common
//! searches. Each takes `[options] <args> [-- <pathspec>…]` and prints hits in
//! the rule output format; this module holds what they share.

use crate::command;
use crate::render;

mod agent_files;
mod cargo;
mod clippy;
mod gdscan;
mod gdtoolkit;
mod grep;
mod manifest;
mod rust_docs;
mod rustfmt;

/// A finder: its arguments after the name, to its hits or a usage error.
type Finder = fn(&[String]) -> Result<Vec<command::Hit>, String>;

/// How `--only-at` limits a fix: the granularity of locations it accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OnlyAtGranularity {
    /// `path:line:col` (grep, gdscan spacing)
    PathLineCol,
    /// `path:line` (gdtoolkit format, rustfmt)
    PathLine,
    /// `path` only (agent-files)
    Path,
}

/// What a finder declares about itself.
pub struct FinderCapability {
    /// The finder's name (e.g., "grep").
    pub name: &'static str,
    /// The finder function.
    pub run: Finder,
    /// Whether a narrowed run can pass `--files-from` to this finder.
    pub narrowable: bool,
    /// The subcommands/checks that are fixable. Empty means all/always fixable.
    pub fixable_checks: &'static [&'static str],
    /// The option words that enable fix mode (e.g., "--replace", "format", "spacing").
    /// Empty means the finder is always fixable (like rustfmt).
    pub fix_requires_option: &'static [&'static str],
    /// How `--only-at` limits fixes for this finder.
    pub only_at_granularity: OnlyAtGranularity,
}

/// The finder registry: all finders with their declared capabilities.
pub const FINDER_REGISTRY: &[FinderCapability] = &[
    FinderCapability {
        name: "agent-files",
        run: agent_files::run,
        narrowable: false,
        fixable_checks: &["claude-md", "skill-link"],
        fix_requires_option: &[],
        only_at_granularity: OnlyAtGranularity::Path,
    },
    FinderCapability {
        name: "cargo-manifest",
        run: manifest::run,
        narrowable: false,
        fixable_checks: &[],
        fix_requires_option: &[],
        only_at_granularity: OnlyAtGranularity::PathLineCol,
    },
    FinderCapability {
        name: "clippy",
        run: clippy::run,
        narrowable: false,
        fixable_checks: &[],
        fix_requires_option: &[],
        only_at_granularity: OnlyAtGranularity::PathLineCol,
    },
    FinderCapability {
        name: "gdscan",
        run: gdscan::run,
        narrowable: true,
        fixable_checks: &["spacing"],
        fix_requires_option: &["spacing"],
        only_at_granularity: OnlyAtGranularity::PathLineCol,
    },
    FinderCapability {
        name: "gdtoolkit",
        run: gdtoolkit::run,
        narrowable: true,
        fixable_checks: &["format"],
        fix_requires_option: &["format"],
        only_at_granularity: OnlyAtGranularity::PathLine,
    },
    FinderCapability {
        name: "grep",
        run: grep::run,
        narrowable: true,
        fixable_checks: &["grep"],
        fix_requires_option: &["--replace"],
        only_at_granularity: OnlyAtGranularity::PathLineCol,
    },
    FinderCapability {
        name: "rust-docs",
        run: rust_docs::run,
        narrowable: true,
        fixable_checks: &[],
        fix_requires_option: &[],
        only_at_granularity: OnlyAtGranularity::PathLineCol,
    },
    FinderCapability {
        name: "rustfmt",
        run: rustfmt::run,
        narrowable: false,
        fixable_checks: &["rustfmt"],
        fix_requires_option: &[],
        only_at_granularity: OnlyAtGranularity::PathLine,
    },
];

/// Find a finder by name.
pub fn find_finder(name: &str) -> Option<&FinderCapability> {
    FINDER_REGISTRY.iter().find(|f| f.name == name)
}

/// Check if a finder is narrowable (accepts --files-from).
pub fn is_narrowable(name: &str) -> bool {
    find_finder(name).map(|f| f.narrowable).unwrap_or(false)
}

/// Check if a finder's command is fixable, given the finder name and the words after it.
pub fn is_fixable(finder: &str, words: &[String]) -> bool {
    let Some(cap) = find_finder(finder) else {
        return false;
    };
    if cap.fixable_checks.is_empty() {
        // Always fixable (like rustfmt) or never fixable (like clippy)
        return cap.name == "rustfmt";
    }
    // For finders where the finder itself is the check (grep, rustfmt),
    // the check name is the finder name.
    // For finders with subcommands (gdscan, gdtoolkit, agent-files),
    // the check name is the first word that is a known subcommand/check.
    let is_self_check = cap.name == "grep" || cap.name == "rustfmt";
    let check = if is_self_check {
        Some(cap.name)
    } else {
        words
            .iter()
            .find(|w| cap.fixable_checks.contains(&w.as_str()))
            .map(String::as_str)
    };
    let Some(check) = check else {
        return false;
    };
    if !cap.fixable_checks.contains(&check) {
        return false;
    }
    if cap.fix_requires_option.is_empty() {
        return true;
    }
    // Check if any of the fix-required options are present in the words
    // For self-check finders (grep), check all words for --replace flag.
    // For subcommand finders (gdscan, gdtoolkit), the subcommand name IS the fix mode,
    // so we just need to verify the check is in fixable_checks (done above).
    // For agent-files, the check is the fix mode and no extra option needed.
    if is_self_check {
        words.iter().any(|w| {
            cap.fix_requires_option
                .iter()
                .any(|opt| w == *opt || w.starts_with(&format!("{opt}=")))
        })
    } else {
        true
    }
}

/// Get the --only-at granularity for a finder.
pub fn only_at_granularity(finder: &str) -> OnlyAtGranularity {
    find_finder(finder)
        .map(|f| f.only_at_granularity)
        .unwrap_or(OnlyAtGranularity::PathLineCol)
}

/// Remove the Cargo finders' build folders: each as (folder, size).
pub fn clean_cache() -> Result<Vec<(String, String)>, String> {
    cargo::clean_cache()
}

/// Sort hit lines by path, line and column as numbers (spec: Finders), not
/// as text, where line 12 would come before line 3. Stable, so lines that
/// don't parse as hits keep their order.
fn sort_hits(hits: &mut [String]) {
    hits.sort_by_cached_key(|h| crate::command::parse(h).map(|hit| (hit.path, hit.line, hit.col)));
}

/// Run `args[0]` as a finder, if it names one: the exit code.
pub fn dispatch(args: &[String]) -> Option<u8> {
    let (name, rest) = args.split_first()?;
    let cap = find_finder(name)?;
    Some(match (cap.run)(rest) {
        Ok(hits) => {
            // Convert command::Hit to strings for output
            let mut hit_strings: Vec<String> = hits.iter().map(|h| h.to_string()).collect();
            sort_hits(&mut hit_strings);
            render::lines(&hit_strings);
            0
        }
        Err(message) => {
            render::usage(&format!("{name}: {message}"));
            2
        }
    })
}

/// Split finder arguments at `--`: (before, pathspecs).
pub fn split_pathspecs(args: &[String]) -> (&[String], &[String]) {
    match args.iter().position(|a| a == "--") {
        Some(i) => (&args[..i], &args[i + 1..]),
        None => (args, &[]),
    }
}

/// Tracked text files matching the pathspecs, sorted; with `containing`,
/// only those holding one of the strings. `git grep` lists them: unlike
/// `git ls-files`, it honours an exclude starting with a wildcard
/// (`:!*x*.gd`). The baseline snapshot is skipped, as its fingerprints would
/// match the rules they came from.
pub fn tracked(pathspecs: &[String], containing: Option<&[String]>) -> Result<Vec<String>, String> {
    let mut git = crate::files::git();
    git.args(["grep", "-l", "-z", "-I"]);
    match containing {
        Some(texts) => {
            git.arg("-F");
            for text in texts {
                git.args(["-e", text]);
            }
        }
        None => {
            git.args(["-e", ""]);
        }
    }
    let out = git
        .arg("--")
        .args(pathspecs)
        .output()
        .map_err(|e| format!("can't run git: {e}"))?;
    if !matches!(out.status.code(), Some(0 | 1)) {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(err.lines().next().unwrap_or("git grep failed").to_string());
    }
    let skip = snapshot_names();
    let mut files: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .split('\0')
        .filter(|p| !p.is_empty())
        .map(String::from)
        .filter(|p| !skip.iter().any(|s| p == s))
        .collect();
    files.sort();
    files.dedup();
    Ok(files)
}

/// The snapshot files `tracked` must skip: the default plus the `baseline:` pointer.
fn snapshot_names() -> Vec<String> {
    let mut names = vec![crate::config::DEFAULT_BASELINE.to_string()];
    let root = std::env::current_dir()
        .ok()
        .and_then(|cwd| crate::files::find_root(&cwd));
    let Some(root) = root else { return names };
    let Ok(text) = std::fs::read_to_string(root.join("necro.yaml")) else {
        return names;
    };
    let Ok(node) = crate::yaml::parse(&text) else {
        return names;
    };
    if let crate::yaml::Value::Map(items) = &node.value {
        for (key, value) in items {
            if key.name == "baseline"
                && let Some(pointer) = value.as_str()
            {
                let pointer = pointer.strip_prefix("./").unwrap_or(pointer);
                if !pointer.is_empty() && !names.contains(&pointer.to_string()) {
                    names.push(pointer.to_string());
                }
            }
        }
    }
    names
}

/// A file's lines, or `None` for a binary file (one with a NUL byte) or one that can't be read.
pub fn text_lines(path: &str) -> Option<Vec<String>> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.contains(&0) {
        return None;
    }
    Some(
        String::from_utf8_lossy(&bytes)
            .lines()
            .map(|l| l.trim_end_matches('\r').to_string())
            .collect(),
    )
}

/// A regex error in a few words: "unclosed group" from fancy_regex's
/// "Parsing error at position 1: Opening parenthesis without closing
/// parenthesis".
pub fn regex_error(e: &fancy_regex::Error) -> String {
    let full = e.to_string();
    let last = full.lines().last().unwrap_or("").trim();
    let last = last.strip_prefix("error: ").unwrap_or(last);
    let short = match last.split_once(": ") {
        Some((head, rest)) if head.starts_with("Parsing error") => rest,
        _ => last,
    };
    let mut chars = short.chars();
    chars
        .next()
        .map(|c| c.to_lowercase().chain(chars).collect())
        .unwrap_or_default()
}
