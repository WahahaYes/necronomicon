//! Exceptions written in the code (spec: Allows): `necro-allow <rule>: <reason>`
//! on a raw line, in any comment style, found with `git grep`.

use std::path::Path;
use std::process::Command;

const ALLOW: &str = "necro-allow";

pub struct Allow {
    pub file: String,
    pub line: usize,
    /// Where `necro-allow` starts, in code points.
    pub col: usize,
    pub rule: String,
    pub reason: Option<String>,
    /// Whether the file starts with `#!`, so a whole-file allow sits on line 2.
    pub shebang: bool,
}

/// Every allow in the tracked files under `root` that match `pathspecs` (all when empty).
pub fn find(root: &Path, pathspecs: &[String]) -> Result<Vec<Allow>, String> {
    let out = Command::new("git")
        .args(["grep", "-l", "-z", "-I", "-F", "-e", ALLOW, "--"])
        .args(pathspecs)
        .current_dir(root)
        .output()
        .map_err(|e| format!("can't run git grep: {e}"))?;
    if !matches!(out.status.code(), Some(0 | 1)) {
        return Err(format!(
            "git grep for allows failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    let mut allows = Vec::new();
    for file in String::from_utf8_lossy(&out.stdout).split('\0') {
        if file.is_empty() {
            continue;
        }
        let Ok(bytes) = std::fs::read(root.join(file)) else {
            continue;
        };
        let text = String::from_utf8_lossy(&bytes);
        let shebang = text.starts_with("#!");
        for (i, line) in text.lines().enumerate() {
            for (col, rule, reason) in parse_line(line) {
                allows.push(Allow {
                    file: file.to_string(),
                    line: i + 1,
                    col,
                    rule,
                    reason,
                    shebang,
                });
            }
        }
    }
    Ok(allows)
}

/// The allows on one line: (column, rule, reason). Each reason runs to the
/// line's end or the next allow, with a trailing `*/`, `-->` or `;` dropped.
fn parse_line(line: &str) -> Vec<(usize, String, Option<String>)> {
    let starts: Vec<usize> = line.match_indices(ALLOW).map(|(i, _)| i).collect();
    let mut out = Vec::new();
    for (k, &start) in starts.iter().enumerate() {
        let end = starts.get(k + 1).copied().unwrap_or(line.len());
        let body = &line[start + ALLOW.len()..end];
        let trimmed = body.trim_start();
        let rule: String = trimmed
            .chars()
            .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-' || *c == '/')
            .collect();
        if rule.is_empty() || !body.starts_with(char::is_whitespace) {
            continue;
        }
        let reason = trimmed[rule.len()..]
            .strip_prefix(':')
            .map(trim_reason)
            .filter(|r| !r.is_empty());
        out.push((line[..start].chars().count() + 1, rule, reason));
    }
    out
}

fn trim_reason(r: &str) -> String {
    let mut r = r.trim();
    loop {
        let next = r
            .trim_end_matches(';')
            .trim_end_matches("*/")
            .trim_end_matches("-->")
            .trim_end();
        if next == r {
            return r.to_string();
        }
        r = next;
    }
}
