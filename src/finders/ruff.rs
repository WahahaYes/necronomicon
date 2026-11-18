//! `necro ruff` (spec: Finders): ruff's diagnostics and formatting over
//! tracked `.py` and `.pyi` files, as hits. ruff comes from `PATH`;
//! `necronomicon[python]` installs the pinned version beside necro.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

use super::grep::read_list;
use super::known;
use crate::args::{self, Spec};
use crate::command::Hit;

const USAGE: &str = "necro ruff [--files-from <file>] [--write [--only-at <file>]] \
                     (check (<code>… | --others [--except=<code>,…]) | format) [-- <pathspec>…]";

/// The codes and prefixes every check run adds to the repo's selection, the
/// ones necro's packs use. One fixed set keeps ruff's arguments the same for
/// every rule, and makes a `noqa` for any of them count as used.
const CODES: &[&str] = &[
    "ANN001", "ANN201", "B904", "C901", "D100", "D101", "D102", "D103", "E711", "E721", "F403",
    "F405", "G004", "PGH003", "PGH004", "PTH",
];

/// The code ruff gives a file it can't parse.
const SYNTAX: &str = "invalid-syntax";

/// Files per ruff run, so a large repo stays under the argument limit.
const CHUNK: usize = 400;

/// What a `check` reports.
enum Wanted {
    /// The given codes, each in the set or under one of its prefixes.
    Codes(Vec<String>),
    /// Everything outside the set and these codes or prefixes.
    Others(Vec<String>),
}

/// The `ruff` finder.
pub fn run(args: &[String]) -> Result<Vec<Hit>, String> {
    let parsed = args::parse(
        args,
        &Spec {
            values: &["--files-from", "--only-at", "--except"],
            flags: &["--write", "--others"],
            pathspecs: true,
        },
    )
    .map_err(|e| e.with_usage(USAGE))?;
    let (check, codes) = parsed
        .positional
        .split_first()
        .ok_or_else(|| format!("name a check: {USAGE}"))?;
    let write = parsed.flag("--write");
    let only_at = parsed.value("--only-at");
    let others = parsed.flag("--others");
    let except: Vec<String> = parsed
        .values("--except")
        .into_iter()
        .flat_map(|list| list.split(',').map(|c| c.trim().to_string()))
        .filter(|c| !c.is_empty())
        .collect();
    let wanted = match check.as_str() {
        "check" => Some(wanted(codes, others, except)?),
        "format" => {
            if let Some(extra) = codes.first() {
                return Err(format!("unexpected argument {extra}: {USAGE}"));
            }
            if others || !except.is_empty() {
                return Err(format!("--others and --except go with check: {USAGE}"));
            }
            None
        }
        other => return Err(format!("unknown check {other}: {USAGE}")),
    };
    if (write || only_at.is_some()) && wanted.is_some() {
        return Err(format!("--write and --only-at go with format: {USAGE}"));
    }
    let mut only = parsed.value("--files-from").map(read_list).transpose()?;
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
    let files = py_files(&parsed.pathspecs, only.as_ref())?;
    if files.is_empty() {
        return Ok(vec![]);
    }
    match wanted {
        Some(w) => lint(&files, &w),
        None => format(&files, write),
    }
}

/// The codes a `check` asks for, validated against the set.
fn wanted(codes: &[String], others: bool, except: Vec<String>) -> Result<Wanted, String> {
    if others {
        if !codes.is_empty() {
            return Err(format!("--others takes no codes: {USAGE}"));
        }
        return Ok(Wanted::Others(except));
    }
    if !except.is_empty() {
        return Err(format!("--except goes with --others: {USAGE}"));
    }
    if codes.is_empty() {
        return Err(format!("give at least one code: {USAGE}"));
    }
    for code in codes {
        if !CODES.iter().any(|c| code.starts_with(c)) {
            return Err(format!(
                "{code} isn't one of necro ruff's codes: {}",
                CODES.join(", ")
            ));
        }
    }
    Ok(Wanted::Codes(codes.to_vec()))
}

/// Known `.py` and `.pyi` files matching the pathspecs (all of them when
/// there are none), kept to `only` when given.
fn py_files(pathspecs: &[String], only: Option<&HashSet<String>>) -> Result<Vec<String>, String> {
    let specs = if pathspecs.is_empty() {
        vec!["*.py".to_string(), "*.pyi".to_string()]
    } else {
        pathspecs.to_vec()
    };
    Ok(known(&specs, None)?
        .into_iter()
        .filter(|p| p.ends_with(".py") || p.ends_with(".pyi"))
        .filter(|p| only.is_none_or(|o| o.contains(p)))
        .collect())
}

/// Run ruff over the files in chunks: each chunk's (exit code, stdout, stderr).
fn ruff(args: &[&str], files: &[String]) -> Result<Vec<(i32, String, String)>, String> {
    let mut out = Vec::new();
    for chunk in files.chunks(CHUNK) {
        let output = match Command::new("ruff").args(args).args(chunk).output() {
            Ok(o) => o,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(
                    "ruff isn't on PATH; install necronomicon[python], which pins ruff beside necro"
                        .to_string(),
                );
            }
            Err(e) => return Err(format!("can't run ruff: {e}")),
        };
        out.push((
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }
    Ok(out)
}

/// The diagnostics in ruff's JSON output, or a check error quoting ruff.
fn diagnostics(code: i32, stdout: &str, stderr: &str, ok: &[i32]) -> Result<Vec<Value>, String> {
    let parsed = serde_json::from_str::<Value>(stdout).ok();
    match parsed {
        Some(Value::Array(items)) if ok.contains(&code) => Ok(items),
        _ => {
            let tail: Vec<&str> = stderr
                .lines()
                .chain(stdout.lines())
                .map(str::trim)
                .filter(|l| !l.is_empty() && !l.starts_with("warning: No Python files found"))
                .collect();
            Err(format!("ruff exited {code}: {}", tail.join(" / ")))
        }
    }
}

/// The working folder, resolved, to make ruff's absolute paths relative.
fn here() -> Result<PathBuf, String> {
    std::env::current_dir()
        .and_then(|d| d.canonicalize())
        .map_err(|e| format!("can't read the working folder: {e}"))
}

/// A path ruff printed, relative to the working folder.
fn relative(here: &Path, file: &str) -> String {
    let path = Path::new(file);
    let resolved = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    resolved
        .strip_prefix(here)
        .unwrap_or(&resolved)
        .to_string_lossy()
        .into_owned()
}

/// A number at `key` of a JSON location.
fn at(location: &Value, key: &str) -> usize {
    location
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|n| usize::try_from(n).ok())
        .unwrap_or(0)
}

/// `ruff check`: the repo's selection plus the set, filtered to what's wanted.
fn lint(files: &[String], wanted: &Wanted) -> Result<Vec<Hit>, String> {
    let select = CODES.join(",");
    let args = [
        "check",
        "--output-format",
        "json",
        "--no-fix",
        "--force-exclude",
        "--extend-select",
        &select,
    ];
    let here = here()?;
    let mut hits = Vec::new();
    for (code, stdout, stderr) in ruff(&args, files)? {
        for d in diagnostics(code, &stdout, &stderr, &[0, 1])? {
            let rule = d.get("code").and_then(Value::as_str).unwrap_or(SYNTAX);
            let keep = match wanted {
                Wanted::Codes(codes) => codes.iter().any(|c| rule.starts_with(c.as_str())),
                Wanted::Others(except) => {
                    !CODES.iter().any(|c| rule.starts_with(c))
                        && !except.iter().any(|c| rule.starts_with(c.as_str()))
                }
            };
            if !keep {
                continue;
            }
            let Some(file) = d.get("filename").and_then(Value::as_str) else {
                continue;
            };
            let start = d.get("location").cloned().unwrap_or(Value::Null);
            let end = d.get("end_location").cloned().unwrap_or(Value::Null);
            let (line, col) = (at(&start, "row"), at(&start, "column"));
            // ruff's end column is exclusive; necro's endcol is inclusive.
            let last = at(&end, "column").saturating_sub(1);
            let endcol = (at(&end, "row") == line && last > col).then_some(last);
            let message = d.get("message").and_then(Value::as_str).unwrap_or_default();
            hits.push(Hit {
                path: relative(&here, file),
                line,
                col,
                endcol,
                label: format!("{message} ({rule})"),
            });
        }
    }
    Ok(hits)
}

/// `ruff format --check`, and with `write` `ruff format` on what it would change.
fn format(files: &[String], write: bool) -> Result<Vec<Hit>, String> {
    let args = [
        "format",
        "--check",
        "--force-exclude",
        "--output-format",
        "json",
    ];
    let here = here()?;
    let mut changed = Vec::new();
    for (code, stdout, stderr) in ruff(&args, files)? {
        for d in diagnostics(code, &stdout, &stderr, &[0, 1, 2])? {
            if d.get("code").and_then(Value::as_str) != Some("unformatted") {
                continue;
            }
            if let Some(file) = d.get("filename").and_then(Value::as_str) {
                changed.push(relative(&here, file));
            }
        }
    }
    changed.sort();
    changed.dedup();
    let label = if write && !changed.is_empty() {
        for (code, stdout, stderr) in ruff(&["format", "--force-exclude"], &changed)? {
            if code != 0 {
                return Err(format!(
                    "ruff format exited {code}: {}{}",
                    stdout.trim(),
                    stderr.trim()
                ));
            }
        }
        "reformatted"
    } else {
        "needs reformatting"
    };
    Ok(changed
        .into_iter()
        .map(|path| Hit {
            path,
            line: 0,
            col: 0,
            endcol: None,
            label: label.to_string(),
        })
        .collect())
}
