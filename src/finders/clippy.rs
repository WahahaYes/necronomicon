//! `necro clippy` (spec: Finders): clippy lints as hits, one cached run per
//! workspace for every rule that uses it.

use std::collections::BTreeMap;
use std::process::Command;

use regex::Regex;
use serde_json::Value;

use super::cargo::{Workspace, cache, relative, workspaces};

const USAGE: &str = "necro clippy [--tests] [--match <regex>] [--skip <regex>] <lint>…";

/// Every lint a run sets to warn, the ones necro:rust uses. One fixed list
/// keeps clippy's arguments the same for every rule, so cargo replays its
/// cached result instead of checking again.
const LINTS: &[&str] = &[
    "allow_attributes",
    "allow_attributes_without_reason",
    "cast_possible_truncation",
    "dbg_macro",
    "expect_used",
    "let_underscore_must_use",
    "todo",
    "undocumented_unsafe_blocks",
    "unimplemented",
    "unused_result_ok",
    "unwrap_used",
];

struct Options {
    tests: bool,
    keep: Option<Regex>,
    drop: Option<Regex>,
    lints: Vec<String>,
}

fn parse(args: &[String]) -> Result<Options, String> {
    let mut o = Options {
        tests: false,
        keep: None,
        drop: None,
        lints: Vec::new(),
    };
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--tests" => o.tests = true,
            "--match" | "--skip" => {
                let pattern = it.next().ok_or(format!("{arg} needs a regex: {USAGE}"))?;
                let re =
                    Regex::new(pattern).map_err(|_| format!("`{pattern}` isn't a valid regex"))?;
                if arg == "--match" {
                    o.keep = Some(re);
                } else {
                    o.drop = Some(re);
                }
            }
            a if a.starts_with('-') => return Err(format!("unknown option {a}: {USAGE}")),
            lint if LINTS.contains(&lint) => o.lints.push(lint.to_string()),
            lint => {
                return Err(format!(
                    "{lint} isn't one of necro clippy's lints: {}",
                    LINTS.join(", ")
                ));
            }
        }
    }
    if o.lints.is_empty() {
        return Err(format!("give at least one lint: {USAGE}"));
    }
    Ok(o)
}

/// The `clippy` finder: the given lints' hits in every workspace.
pub fn run(args: &[String]) -> Result<Vec<String>, String> {
    let o = parse(args)?;
    let wanted: Vec<String> = o.lints.iter().map(|l| format!("clippy::{l}")).collect();
    // (path, line) -> (column, location, message): one hit per line.
    let mut hits: BTreeMap<(String, usize), (usize, String, String)> = BTreeMap::new();
    for ws in workspaces()? {
        check(&ws, &o, &wanted, &mut hits)?;
    }
    Ok(hits
        .into_iter()
        .map(|((path, line), (_, col, message))| format!("{path}:{line}:{col}: {message}"))
        .collect())
}

fn check(
    ws: &Workspace,
    o: &Options,
    wanted: &[String],
    hits: &mut BTreeMap<(String, usize), (usize, String, String)>,
) -> Result<(), String> {
    let mut cmd = Command::new("cargo");
    cmd.args(["clippy", "--quiet", "--message-format=json", "--workspace"]);
    if o.tests {
        cmd.arg("--all-targets");
    }
    // Its own target folder: arguments that differ from the repo's own clippy
    // run would otherwise make each recheck what the other just checked.
    cmd.arg("--target-dir").arg(cache(ws));
    cmd.arg("--");
    cmd.args(LINTS.iter().map(|l| format!("-Wclippy::{l}")));
    let out = cmd
        .current_dir(&ws.root)
        .output()
        .map_err(|e| format!("can't run cargo: {e}"))?;
    let mut errors = Vec::new();
    for line in String::from_utf8_lossy(&out.stdout).lines() {
        let Ok(msg) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if msg["reason"] != "compiler-message" {
            continue;
        }
        let diag = &msg["message"];
        if diag["level"] == "error" {
            let text = diag["rendered"].as_str().or(diag["message"].as_str());
            errors.push(text.unwrap_or_default().to_string());
        }
        let code = diag["code"]["code"].as_str().unwrap_or_default();
        if !wanted.iter().any(|w| w == code) {
            continue;
        }
        let Some(span) = user_span(diag) else {
            continue;
        };
        let file = span["file_name"].as_str().unwrap_or_default();
        let Some(path) = relative(&ws.root.join(file)) else {
            continue;
        };
        let text = highlighted(span);
        if o.keep.as_ref().is_some_and(|re| !re.is_match(&text))
            || o.drop.as_ref().is_some_and(|re| re.is_match(&text))
        {
            continue;
        }
        let (line, start, end) = (
            number(&span["line_start"]),
            number(&span["column_start"]),
            number(&span["column_end"]),
        );
        let mut col = start.to_string();
        if line == number(&span["line_end"]) && end > start + 1 {
            col = format!("{start}-{}", end - 1);
        }
        // Two of a rule's lints on one line (allow_attributes and its
        // _without_reason twin) are one violation, at the leftmost column.
        let message = diag["message"].as_str().unwrap_or_default().to_string();
        let slot = hits
            .entry((path, line))
            .or_insert((start, col.clone(), message.clone()));
        if start < slot.0 {
            *slot = (start, col, message);
        }
    }
    if !out.status.success() || !errors.is_empty() {
        let mut text = errors.concat();
        if text.is_empty() {
            text = String::from_utf8_lossy(&out.stderr).trim().to_string();
        }
        return Err(text.trim_end().to_string());
    }
    Ok(())
}

/// The diagnostic's primary span in the repo's code, following macro
/// expansions out of files outside it.
fn user_span(diag: &Value) -> Option<&Value> {
    for first in diag["spans"].as_array()? {
        let mut span = first;
        while span["is_primary"] == true {
            let file = span["file_name"].as_str().unwrap_or_default();
            if !std::path::Path::new(file).is_absolute() {
                return Some(span);
            }
            span = &span["expansion"]["span"];
        }
    }
    None
}

/// The span's source text: the highlighted part of each line it covers.
fn highlighted(span: &Value) -> String {
    let lines = span["text"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_default();
    let parts: Vec<String> = lines
        .iter()
        .map(|t| {
            let text = t["text"].as_str().unwrap_or_default();
            let (from, to) = (number(&t["highlight_start"]), number(&t["highlight_end"]));
            text.chars()
                .skip(from.saturating_sub(1))
                .take(to.saturating_sub(from))
                .collect()
        })
        .collect();
    parts.join("\n").trim().to_string()
}

/// A 1-based position from cargo's JSON; 0 when it's missing.
fn number(v: &Value) -> usize {
    v.as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(0)
}
