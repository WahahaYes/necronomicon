//! `necro gdtoolkit` (spec: Finders): gdformat and gdlint (gdtoolkit) over
//! tracked `.gd` files, their output turned into hits. The tools come from
//! `PATH`; `necronomicon[gdscript]` installs the pinned version beside necro.

use std::collections::HashSet;
use std::process::Command;

use crate::args::{self, Spec};
use crate::command::Hit;
use regex::Regex;

use super::gdscan::{gd_files, has_inline_lambda};
use super::grep::read_list;

const USAGE: &str = "necro gdtoolkit [--files-from <file>] [--line-length <n>] \
                     [--write [--only-at <file>]] (format|lint|parse) [-- <pathspec>…]";

/// The checks `gdtoolkit` knows.
const CHECKS: &[&str] = &["format", "lint", "parse"];

/// Files per tool run, so a large repo stays under the argument limit.
const CHUNK: usize = 400;

/// The `gdtoolkit` finder.
pub fn run(args: &[String]) -> Result<Vec<Hit>, String> {
    let parsed = args::parse(
        args,
        &Spec {
            values: &["--files-from", "--only-at", "--line-length"],
            flags: &["--write"],
            pathspecs: true,
        },
    )
    .map_err(|e| e.with_usage(USAGE))?;
    let list = parsed.value("--files-from");
    let only_at = parsed.value("--only-at");
    let mut line_length = None;
    for v in parsed.values("--line-length") {
        if v.parse::<usize>().map_or(true, |n| n == 0) {
            return Err(format!("--line-length takes a whole number, got {v}"));
        }
        line_length = Some(v);
    }
    let write = parsed.flag("--write");
    let check = args::check(&parsed.positional, CHECKS, USAGE)?;
    if (write || only_at.is_some()) && check != "format" {
        return Err(format!("--write and --only-at go with format: {USAGE}"));
    }
    let mut only = list.map(read_list).transpose()?;
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
    let files = gd_files(&parsed.pathspecs, only.as_ref())?;
    if files.is_empty() {
        return Ok(vec![]);
    }
    let length: Vec<String> = line_length
        .map(|n| vec!["--line-length".to_string(), n.to_string()])
        .unwrap_or_default();
    match check {
        "lint" => lint(&files),
        "parse" => {
            let check = gdformat_check(&files, &length)?;
            Ok(check.parse_errors)
        }
        _ if write => format_write(&files, &length),
        _ => {
            let found = gdformat_check(&files, &length)?;
            Ok(found
                .reformat
                .iter()
                .map(|f| {
                    if has_inline_lambda(f) {
                        Hit {
                            path: f.to_string(),
                            line: 0,
                            col: 0,
                            endcol: None,
                            label: "needs reformatting; --fix skips it, as gdformat corrupts a lambda inside brackets".to_string(),
                        }
                    } else {
                        Hit {
                            path: f.to_string(),
                            line: 0,
                            col: 0,
                            endcol: None,
                            label: "needs reformatting".to_string(),
                        }
                    }
                })
                .collect())
        }
    }
}

/// What `gdformat --check` found.
struct Check {
    reformat: Vec<String>,
    parse_errors: Vec<Hit>,
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
fn parse_blocks(text: &str, files: &HashSet<&str>) -> (Vec<Hit>, Vec<String>) {
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
                hits.push(Hit {
                    path: prev.to_string(),
                    line: 0,
                    col: 0,
                    endcol: None,
                    label: "can't parse".to_string(),
                });
            }
            open = Some(f);
            continue;
        }
        if let Some(f) = open {
            if let Some(c) = pos.captures(t) {
                let message = t.split(" at line").next().unwrap_or(t);
                hits.push(Hit {
                    path: f.to_string(),
                    line: c[1].parse().unwrap_or(0),
                    col: c[2].parse().unwrap_or(0),
                    endcol: None,
                    label: message.to_string(),
                });
                open = None;
            }
            continue;
        }
        rest.push(line.to_string());
    }
    if let Some(f) = open {
        hits.push(Hit {
            path: f.to_string(),
            line: 0,
            col: 0,
            endcol: None,
            label: "can't parse".to_string(),
        });
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
fn lint(files: &[String]) -> Result<Vec<Hit>, String> {
    let known: HashSet<&str> = files.iter().map(String::as_str).collect();
    let Ok(hit) = Regex::new(r"^(.+?):(\d+): Error: (.*)$") else {
        return Ok(vec![]);
    };
    let mut hits = Vec::new();
    for (code, text) in tool("gdlint", &[], files)? {
        let (_errors, rest) = parse_blocks(&text, &known);
        let mut strange = Vec::new();
        let before = hits.len();
        for line in rest {
            match hit.captures(line.trim()) {
                Some(c) => {
                    let line = func_line(&c[1], &c[2], &c[3]).unwrap_or_else(|| c[2].to_string());
                    let line_num: usize = line.parse().unwrap_or(0);
                    hits.push(Hit {
                        path: c[1].to_string(),
                        line: line_num,
                        col: 0,
                        endcol: None,
                        label: c[3].to_string(),
                    });
                }
                None if chatter(&line) => {}
                None => strange.push(line),
            }
        }
        // Don't include parse errors in lint results - they belong to the "parse" check
        if code > 1 || (code == 1 && hits.len() == before) {
            return Err(unexpected("gdlint", code, &strange));
        }
    }
    hits.sort_by(|a, b| a.path.cmp(&b.path).then(a.line.cmp(&b.line)));
    Ok(hits)
}

/// For `max-returns`, which gdlint puts on the function's last `return`, the
/// line of the `func` it names, at or above `line`, so a baseline entry
/// fingerprints the function.
fn func_line(path: &str, line: &str, message: &str) -> Option<String> {
    if !message.ends_with("(max-returns)") {
        return None;
    }
    let name = message.strip_prefix("Function \"")?.split('"').next()?;
    let at: usize = line.parse().ok()?;
    let text = std::fs::read_to_string(path).ok()?;
    let lines: Vec<&str> = text.lines().take(at).collect();
    lines
        .iter()
        .rposition(|l| {
            let l = l.trim_start();
            let l = l.strip_prefix("static ").map_or(l, str::trim_start);
            l.strip_prefix("func ")
                .map(str::trim_start)
                .and_then(|r| r.strip_prefix(name))
                .is_some_and(|r| r.trim_start().starts_with('('))
        })
        .map(|i| (i + 1).to_string())
}

/// `format --write`: reformat the files gdformat would change, except those
/// holding a lambda it would corrupt; the hits for the files it reformatted.
fn format_write(files: &[String], length: &[String]) -> Result<Vec<Hit>, String> {
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
        .map(|f| Hit {
            path: f.to_string(),
            line: 0,
            col: 0,
            endcol: None,
            label: "reformatted".to_string(),
        })
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
