//! `necro gdtoolkit` (spec: Finders): gdformat and gdlint (gdtoolkit) over
//! tracked `.gd` files, their output turned into hits. The tools come from
//! `PATH`; `necronomicon[gdscript]` installs the pinned version beside necro.

use std::collections::HashSet;
use std::process::Command;

use regex::Regex;

use super::gdscan::{gd_files, has_inline_lambda};
use super::grep::read_list;
use super::split_pathspecs;

const USAGE: &str = "necro gdtoolkit [--files-from <file>] [--line-length <n>] \
                     [--write [--only-at <file>]] (format|lint|parse) [-- <pathspec>…]";

/// Files per tool run, so a large repo stays under the argument limit.
const CHUNK: usize = 400;

/// The `gdtoolkit` finder.
pub fn run(args: &[String]) -> Result<Vec<String>, String> {
    let (opts, pathspecs) = split_pathspecs(args);
    let mut list: Option<String> = None;
    let mut only_at: Option<String> = None;
    let mut line_length: Option<String> = None;
    let mut write = false;
    let mut check: Option<&str> = None;
    let mut it = opts.iter();
    while let Some(arg) = it.next() {
        let (flag, inline) = match arg.split_once('=') {
            Some((f, v)) if arg.starts_with("--") => (f, Some(v.to_string())),
            _ => (arg.as_str(), None),
        };
        let mut value = || -> Result<String, String> {
            inline
                .clone()
                .or_else(|| it.next().cloned())
                .ok_or(format!("{flag} needs a value: {USAGE}"))
        };
        match flag {
            "--files-from" => list = Some(value()?),
            "--only-at" => only_at = Some(value()?),
            "--line-length" => {
                let v = value()?;
                if v.parse::<usize>().map_or(true, |n| n == 0) {
                    return Err(format!("--line-length takes a whole number, got {v}"));
                }
                line_length = Some(v);
            }
            "--write" => write = true,
            f if f.starts_with('-') => return Err(format!("unknown option {f}: {USAGE}")),
            c if check.is_none() => check = Some(c),
            other => return Err(format!("unexpected argument {other}: {USAGE}")),
        }
    }
    let check = check.ok_or(format!("name a check: {USAGE}"))?;
    if !["format", "lint", "parse"].contains(&check) {
        return Err(format!("unknown check {check}: {USAGE}"));
    }
    if (write || only_at.is_some()) && check != "format" {
        return Err(format!("--write and --only-at go with format: {USAGE}"));
    }
    let mut only = list.as_deref().map(read_list).transpose()?;
    if let Some(at) = &only_at {
        // `path:line:col` lines; a whole-file hit is just `path`.
        let files: HashSet<String> = read_list(at)?
            .into_iter()
            .map(|l| l.split(':').next().unwrap_or_default().to_string())
            .collect();
        only = Some(match only {
            Some(o) => o.intersection(&files).cloned().collect(),
            None => files,
        });
    }
    let files = gd_files(pathspecs, only.as_ref())?;
    if files.is_empty() {
        return Ok(vec![]);
    }
    let length: Vec<String> = line_length
        .map(|n| vec!["--line-length".to_string(), n])
        .unwrap_or_default();
    match check {
        "lint" => lint(&files),
        "parse" => Ok(gdformat_check(&files, &length)?.parse_errors),
        _ if write => format_write(&files, &length),
        _ => {
            let found = gdformat_check(&files, &length)?;
            Ok(found
                .reformat
                .iter()
                .map(|f| {
                    if has_inline_lambda(f) {
                        format!(
                            "{f}: needs reformatting; --fix skips it, as gdformat corrupts a lambda inside a multi-line bracket"
                        )
                    } else {
                        format!("{f}: needs reformatting")
                    }
                })
                .collect())
        }
    }
}

/// What `gdformat --check` found.
struct Check {
    reformat: Vec<String>,
    parse_errors: Vec<String>,
}

/// Run a gdtoolkit program over the files in chunks: (exit code, stdout and stderr) per chunk.
fn tool(program: &str, args: &[String], files: &[String]) -> Result<Vec<(i32, String)>, String> {
    let mut out = Vec::new();
    for chunk in files.chunks(CHUNK) {
        let result = Command::new(program).args(args).args(chunk).output();
        let output = match result {
            Ok(o) => o,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(format!(
                    "{program} isn't on PATH; install necronomicon[gdscript], which pins gdtoolkit beside necro"
                ));
            }
            Err(e) => return Err(format!("can't run {program}: {e}")),
        };
        let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
        text.push_str(&String::from_utf8_lossy(&output.stderr));
        out.push((output.status.code().unwrap_or(-1), text));
    }
    Ok(out)
}

/// Parse-error blocks in a tool's output: `path:` alone on a line, then
/// the code, then a message with `at line N, column M` (or `col M`).
/// Returns hits and the lines that were part of no block.
fn parse_blocks(text: &str, files: &HashSet<&str>) -> (Vec<String>, Vec<String>) {
    let Ok(pos) = Regex::new(r"line (\d+),? col(?:umn)? (\d+)") else {
        return (vec![], vec![]);
    };
    let mut hits = Vec::new();
    let mut rest = Vec::new();
    let mut open: Option<&str> = None;
    for line in text.lines() {
        let t = line.trim();
        if let Some(f) = t.strip_suffix(':').filter(|f| files.contains(f)) {
            if let Some(prev) = open {
                hits.push(format!("{prev}: can't parse"));
            }
            open = Some(f);
            continue;
        }
        if let Some(f) = open {
            if let Some(c) = pos.captures(t) {
                let message = t.split(" at line").next().unwrap_or(t);
                hits.push(format!("{f}:{}:{}: {message}", &c[1], &c[2]));
                open = None;
            }
            continue;
        }
        rest.push(line.to_string());
    }
    if let Some(f) = open {
        hits.push(format!("{f}: can't parse"));
    }
    (hits, rest)
}

/// Lines that are tool chatter, not findings.
fn chatter(line: &str) -> bool {
    let t = line.trim();
    t.is_empty()
        || t.starts_with("Success:")
        || t.starts_with("Failure:")
        || t.starts_with("Expected one of")
        || t.starts_with("* ")
        || t.starts_with("Previous tokens")
        || t.contains("would be left unchanged")
        || t.contains("would be reformatted")
        || t.contains("reformatted,")
        || t.contains("file reformatted")
        || t.contains("files reformatted")
        || t.contains("left unchanged")
}

/// `gdformat --check`: the files it would change, and parse errors.
fn gdformat_check(files: &[String], length: &[String]) -> Result<Check, String> {
    let known: HashSet<&str> = files.iter().map(String::as_str).collect();
    let mut args = vec!["--check".to_string()];
    args.extend(length.iter().cloned());
    let mut check = Check {
        reformat: vec![],
        parse_errors: vec![],
    };
    for (code, text) in tool("gdformat", &args, files)? {
        let (errors, rest) = parse_blocks(&text, &known);
        check.parse_errors.extend(errors);
        let mut strange = Vec::new();
        for line in rest {
            match line.trim().strip_prefix("would reformat ") {
                Some(f) => check.reformat.push(f.to_string()),
                None if chatter(&line) => {}
                None => strange.push(line),
            }
        }
        if code > 1 || (code == 1 && check.reformat.is_empty() && check.parse_errors.is_empty()) {
            return Err(unexpected("gdformat", code, &strange));
        }
    }
    check.reformat.sort();
    check.parse_errors.sort();
    Ok(check)
}

/// `gdlint`: `path:line: Error: <message> (<check>)`, parse errors left to `parse`.
fn lint(files: &[String]) -> Result<Vec<String>, String> {
    let known: HashSet<&str> = files.iter().map(String::as_str).collect();
    let Ok(hit) = Regex::new(r"^(.+?):(\d+): Error: (.*)$") else {
        return Ok(vec![]);
    };
    let mut hits = Vec::new();
    for (code, text) in tool("gdlint", &[], files)? {
        let (errors, rest) = parse_blocks(&text, &known);
        let mut strange = Vec::new();
        let before = hits.len();
        for line in rest {
            match hit.captures(line.trim()) {
                Some(c) => hits.push(format!("{}:{}: {}", &c[1], &c[2], &c[3])),
                None if chatter(&line) => {}
                None => strange.push(line),
            }
        }
        if code > 1 || (code == 1 && hits.len() == before && errors.is_empty()) {
            return Err(unexpected("gdlint", code, &strange));
        }
    }
    hits.sort();
    Ok(hits)
}

/// `format --write`: reformat the files gdformat would change, except those
/// holding a lambda it would corrupt; the hits for the files it reformatted.
fn format_write(files: &[String], length: &[String]) -> Result<Vec<String>, String> {
    let found = gdformat_check(files, length)?;
    let safe: Vec<String> = found
        .reformat
        .into_iter()
        .filter(|f| !has_inline_lambda(f))
        .collect();
    if safe.is_empty() {
        return Ok(vec![]);
    }
    for (code, text) in tool("gdformat", length, &safe)? {
        if code != 0 {
            let lines: Vec<String> = text.lines().map(String::from).collect();
            return Err(unexpected("gdformat", code, &lines));
        }
    }
    Ok(safe
        .into_iter()
        .map(|f| format!("{f}: reformatted"))
        .collect())
}

/// A tool failure necro can't attribute to a file.
fn unexpected(program: &str, code: i32, lines: &[String]) -> String {
    let tail: Vec<&str> = lines
        .iter()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();
    format!(
        "{program} exited {code} with output necro doesn't recognize: {}",
        tail.join(" / ")
    )
}
