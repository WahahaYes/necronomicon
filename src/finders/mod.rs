//! Finders (spec: Finders): ordinary commands necro bundles for common
//! searches. Each takes `[options] <args> [-- <pathspec>…]` and prints hits in
//! the rule output format; this module holds what they share.

use std::process::Command;

use crate::render;

mod grep;

/// A finder: its arguments after the name, to its hits or a usage error.
type Finder = fn(&[String]) -> Result<Vec<String>, String>;

/// The finders by name, for dispatch from the CLI.
const FINDERS: &[(&str, Finder)] = &[("grep", grep::run)];

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

/// Tracked files matching the pathspecs under the working folder, sorted.
pub fn tracked(pathspecs: &[String]) -> Result<Vec<String>, String> {
    let out = Command::new("git")
        .args(["ls-files", "-z", "--"])
        .args(pathspecs)
        .output()
        .map_err(|e| format!("can't run git: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(err
            .lines()
            .next()
            .unwrap_or("git ls-files failed")
            .to_string());
    }
    let mut files: Vec<String> = String::from_utf8_lossy(&out.stdout)
        .split('\0')
        .filter(|p| !p.is_empty())
        .map(String::from)
        .collect();
    files.sort();
    files.dedup();
    Ok(files)
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
