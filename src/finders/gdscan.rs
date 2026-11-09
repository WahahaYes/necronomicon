//! `necro gdscan` (spec: Finders): GDScript layout checks that track state
//! across lines, read without parsing GDScript. One check per run.

use std::collections::HashSet;

use super::gdscript::{self, Kind, Source};
use super::grep::read_list;
use super::{split_pathspecs, tracked};

const USAGE: &str = "necro gdscan [--files-from <file>] [--header <n>] [--item <n>] \
                     [--write [--only-at <file>]] (spacing|inline-lambda|doc-length|export-doc|uid) [-- <pathspec>…]";

/// What `doc-length` allows when `--header` or `--item` isn't given.
const HEADER: usize = 10;
const ITEM: usize = 6;

/// Lines that may come before a file's header doc.
const PREAMBLE: &[&str] = &["@tool", "@icon", "class_name ", "extends "];

/// The `gdscan` finder.
pub fn run(args: &[String]) -> Result<Vec<String>, String> {
    let (opts, pathspecs) = split_pathspecs(args);
    let (mut header, mut item) = (HEADER, ITEM);
    let mut list: Option<String> = None;
    let mut only_at: Option<String> = None;
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
            "--write" => write = true,
            "--header" => header = lines_arg(flag, &value()?)?,
            "--item" => item = lines_arg(flag, &value()?)?,
            f if f.starts_with('-') => return Err(format!("unknown option {f}: {USAGE}")),
            c if check.is_none() => check = Some(c),
            other => return Err(format!("unexpected argument {other}: {USAGE}")),
        }
    }
    let check = check.ok_or(format!("name a check: {USAGE}"))?;
    if (write || only_at.is_some()) && check != "spacing" {
        return Err(format!("--write goes with spacing: {USAGE}"));
    }
    let only = list.as_deref().map(read_list).transpose()?;
    if check == "uid" {
        return uid(pathspecs, only.as_ref());
    }
    let run: fn(&str, &Source, usize, usize, &mut Vec<String>) = match check {
        "spacing" => spacing,
        "inline-lambda" => inline_lambda,
        "doc-length" => doc_length,
        "export-doc" => export_doc,
        other => return Err(format!("unknown check {other}: {USAGE}")),
    };
    let markers = ["#".to_string()];
    let mut hits = Vec::new();
    for path in gd_files(pathspecs, only.as_ref())? {
        if let Some(src) = gdscript::scan_file(&path, &markers) {
            run(&path, &src, header, item, &mut hits);
        }
    }
    if !write {
        return Ok(hits);
    }
    let at: Option<HashSet<String>> = only_at.as_deref().map(read_list).transpose()?.map(|l| {
        l.iter()
            .map(|h| h.splitn(3, ':').take(2).collect::<Vec<_>>().join(":"))
            .collect()
    });
    write_spacing(&hits, at.as_ref())
}

/// `spacing --write`: for each hit's block, join a top-level comment that
/// blank lines keep from its function (gdformat would widen that gap and
/// detach a `##` doc), then make two blank lines above the block. Bottom up,
/// so earlier line numbers hold, in the file's own line endings.
fn write_spacing(hits: &[String], at: Option<&HashSet<String>>) -> Result<Vec<String>, String> {
    let mut by_file: std::collections::BTreeMap<&str, Vec<usize>> = Default::default();
    for hit in hits {
        let mut parts = hit.splitn(3, ':');
        let (Some(path), Some(line)) = (parts.next(), parts.next()) else {
            continue;
        };
        if at.is_some_and(|a| !a.contains(&format!("{path}:{line}"))) {
            continue;
        }
        if let Ok(line) = line.parse::<usize>() {
            by_file.entry(path).or_default().push(line);
        }
    }
    let mut fixed = Vec::new();
    for (path, mut blocks) in by_file {
        let text = std::fs::read_to_string(path).map_err(|e| format!("can't read {path}: {e}"))?;
        let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
        let mut lines: Vec<&str> = text.split_inclusive('\n').collect();
        let blank = |l: &str| l.trim().is_empty();
        blocks.sort_unstable_by(|a, b| b.cmp(a));
        for &line in &blocks {
            let mut start = (line - 1).min(lines.len());
            let mut gap = start;
            while gap > 0 && blank(lines[gap - 1]) {
                gap -= 1;
            }
            if gap > 0 && gap < start && lines[gap - 1].starts_with('#') {
                lines.drain(gap..start);
                start = gap;
                while start > 0 && lines[start - 1].starts_with('#') {
                    start -= 1;
                }
            }
            let above = lines[..start].iter().rev().take_while(|l| blank(l)).count();
            for _ in above..2 {
                lines.insert(start, eol);
            }
        }
        std::fs::write(path, lines.concat()).map_err(|e| format!("can't write {path}: {e}"))?;
        blocks.reverse();
        fixed.extend(blocks.iter().map(|line| format!("{path}:{line}:1: fixed")));
    }
    Ok(fixed)
}

/// A line limit: a whole number, at least 1.
fn lines_arg(flag: &str, value: &str) -> Result<usize, String> {
    match value.parse::<usize>() {
        Ok(n) if n > 0 => Ok(n),
        _ => Err(format!("{flag} takes a whole number of lines, got {value}")),
    }
}

/// Tracked `.gd` files matching the pathspecs (all of them when there are
/// none), kept to `--files-from`'s list when given.
pub(super) fn gd_files(
    pathspecs: &[String],
    only: Option<&HashSet<String>>,
) -> Result<Vec<String>, String> {
    let specs = if pathspecs.is_empty() {
        vec!["*.gd".to_string()]
    } else {
        pathspecs.to_vec()
    };
    Ok(tracked(&specs, None)?
        .into_iter()
        .filter(|p| p.ends_with(".gd"))
        .filter(|p| only.is_none_or(|o| o.contains(p)))
        .collect())
}

/// Leading lines that may come before the header doc: blanks, `#!`, `@tool`, `extends`, ….
fn preamble_len(lines: &[String]) -> usize {
    lines
        .iter()
        .enumerate()
        .take_while(|(n, l)| {
            let t = l.trim();
            t.is_empty()
                || (*n == 0 && t.starts_with("#!"))
                || PREAMBLE.iter().any(|p| t.starts_with(p))
        })
        .count()
}

/// `spacing`: a top-level func block (with its attached comments) needs 2 blank lines before it.
fn spacing(path: &str, src: &Source, _: usize, _: usize, hits: &mut Vec<String>) {
    let lines = &src.lines;
    let mut previous: Option<usize> = None;
    for (i, &depth) in src.depth.iter().enumerate() {
        let raw = &lines[i];
        if depth > 0
            || src.in_string[i]
            || !(raw.starts_with("func ") || raw.starts_with("static func "))
        {
            continue;
        }
        let mut start = i;
        while start > 0 && lines[start - 1].starts_with('#') {
            start -= 1;
        }
        let blank = lines[..start]
            .iter()
            .rev()
            .take_while(|l| l.trim().is_empty())
            .count();
        if let Some(p) = previous
            && blank < 2
        {
            let s = if blank == 1 { "" } else { "s" };
            hits.push(format!(
                "{path}:{}:1: {blank} blank line{s}; the previous block starts at line {p}",
                start + 1
            ));
        }
        previous = Some(start + 1);
    }
}

/// `inline-lambda`: a lambda inside brackets opened on an earlier line (gdformat corrupts these).
fn inline_lambda(path: &str, src: &Source, _: usize, _: usize, hits: &mut Vec<String>) {
    for (n, col, end) in lambda_spans(src) {
        hits.push(format!("{path}:{n}:{col}-{end}"));
    }
}

/// Each lambda literal inside an open bracket, whether it opened on this line
/// or an earlier one: (line, col, endcol), 1-based. Strings and comments skip.
fn lambda_spans(src: &Source) -> Vec<(usize, usize, usize)> {
    let mut out = Vec::new();
    for (n, line) in src.lines.iter().enumerate() {
        if src.in_string[n] {
            continue;
        }
        let mut depth = src.depth[n];
        let mut prev_word = false;
        for span in &src.spans[n] {
            match span.kind {
                Kind::Comment => break,
                Kind::String => prev_word = false,
                Kind::Code => {
                    let code = &line[span.start..span.end];
                    let base = line[..span.start].chars().count();
                    for (i, (at, c)) in code.char_indices().enumerate() {
                        let col = base + i;
                        if "([{".contains(c) {
                            depth += 1;
                        } else if ")]}".contains(c) {
                            depth = depth.saturating_sub(1);
                        } else if depth > 0
                            && !prev_word
                            && let Some(len) = lambda_len(&code[at..])
                        {
                            out.push((n + 1, col + 1, col + len));
                        }
                        prev_word = c.is_alphanumeric() || c == '_';
                    }
                }
            }
        }
    }
    out
}

/// The length in chars of a lambda's `func` and `(`, with any space between.
fn lambda_len(rest: &str) -> Option<usize> {
    let tail = rest.strip_prefix("func")?;
    let args = tail.trim_start();
    if !args.starts_with('(') {
        return None;
    }
    Some("func".len() + tail[..tail.len() - args.len()].chars().count() + 1)
}

/// Whether a file holds a lambda that gdformat would corrupt.
pub(super) fn has_inline_lambda(path: &str) -> bool {
    let markers = ["#".to_string()];
    gdscript::scan_file(path, &markers).is_some_and(|src| !lambda_spans(&src).is_empty())
}

/// `doc-length`: a run of own-line `##` comments over `item` lines, or
/// `header` for the first one right after the preamble.
fn doc_length(path: &str, src: &Source, header: usize, item: usize, hits: &mut Vec<String>) {
    let preamble = preamble_len(&src.lines);
    // (first line, column, count)
    let mut blocks: Vec<(usize, usize, usize)> = Vec::new();
    let mut last: Option<usize> = None;
    for c in src
        .comments
        .iter()
        .filter(|c| c.own_line && c.doc && c.line >= preamble)
    {
        match blocks.last_mut() {
            Some(b) if last == Some(c.line.wrapping_sub(1)) => b.2 = c.line - b.0 + 1,
            _ => blocks.push((c.line, c.col, 1)),
        }
        last = Some(c.line);
    }
    for (i, (line, col, count)) in blocks.into_iter().enumerate() {
        let limit = if i == 0 && line == preamble {
            header
        } else {
            item
        };
        if count > limit {
            hits.push(format!(
                "{path}:{}:{}: {count} lines, over {limit}",
                line + 1,
                col + 1
            ));
        }
    }
}

/// `export-doc`: an `@export` with no own-line `##` comment directly above it.
fn export_doc(path: &str, src: &Source, _: usize, _: usize, hits: &mut Vec<String>) {
    let documented: HashSet<usize> = src
        .comments
        .iter()
        .filter(|c| c.own_line && c.doc)
        .map(|c| c.line)
        .collect();
    for (n, line) in src.code.iter().enumerate() {
        let t = line.trim_start();
        if !t.starts_with("@export")
            || t[7..].starts_with(|c: char| c.is_alphanumeric() || c == '_')
        {
            continue;
        }
        if n == 0 || !documented.contains(&(n - 1)) {
            let col = line[..line.len() - t.len()].chars().count() + 1;
            hits.push(format!("{path}:{}:{col}-{}", n + 1, col + 6));
        }
    }
}

/// `uid`: in a repo that tracks `.gd.uid` files (Godot 4.4 and later),
/// every tracked `.gd` has its tracked `.uid`, and every `.uid` its `.gd`.
fn uid(pathspecs: &[String], only: Option<&HashSet<String>>) -> Result<Vec<String>, String> {
    let specs = if pathspecs.is_empty() {
        vec!["*.gd".to_string(), "*.gd.uid".to_string()]
    } else {
        pathspecs.to_vec()
    };
    let files = tracked(&specs, None)?;
    let gd: HashSet<&str> = files
        .iter()
        .filter(|p| p.ends_with(".gd"))
        .map(String::as_str)
        .collect();
    let uids: HashSet<&str> = files
        .iter()
        .filter_map(|p| p.strip_suffix(".uid").filter(|g| g.ends_with(".gd")))
        .collect();
    if uids.is_empty() {
        return Ok(vec![]);
    }
    let wanted = |p: &str| only.is_none_or(|o| o.contains(p));
    let mut hits: Vec<String> = gd
        .iter()
        .filter(|g| !uids.contains(*g) && wanted(g))
        .map(|g| format!("{g}: no tracked {g}.uid"))
        .chain(
            uids.iter()
                .filter(|g| !gd.contains(*g) && wanted(&format!("{g}.uid")))
                .map(|g| format!("{g}.uid: no tracked {g}")),
        )
        .collect();
    hits.sort();
    Ok(hits)
}
