//! Finders (spec: Finders): ordinary commands necro bundles for common
//! searches. Each takes `[options] <args> [-- <pathspec>…]` and prints hits in
//! the rule output format; this module holds what they share.

use crate::render;

mod cargo;
mod clippy;
mod grep;
mod manifest;

/// A finder: its arguments after the name, to its hits or a usage error.
type Finder = fn(&[String]) -> Result<Vec<String>, String>;

/// The finders by name, for dispatch from the CLI.
const FINDERS: &[(&str, Finder)] = &[
    ("cargo-manifest", manifest::run),
    ("clippy", clippy::run),
    ("grep", grep::run),
];

/// Run `args[0]` as a finder, if it names one: the exit code.
pub fn dispatch(args: &[String]) -> Option<u8> {
    let (name, rest) = args.split_first()?;
    let (_, run) = FINDERS.iter().find(|(n, _)| n == name)?;
    Some(match run(rest) {
        Ok(hits) => {
            render::lines(&hits);
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

/// Tracked text files matching the pathspecs under the working folder,
/// sorted. Listed by `git grep`, not `git ls-files`, which ignores an exclude
/// pathspec that starts with a wildcard (`:!*x*.gd`). The baseline snapshot
/// is skipped: its plain-text fingerprints would match the content rules they
/// were taken from.
pub fn tracked(pathspecs: &[String]) -> Result<Vec<String>, String> {
    let out = crate::files::git()
        .args(["grep", "-l", "-z", "-I", "-e", "", "--"])
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
