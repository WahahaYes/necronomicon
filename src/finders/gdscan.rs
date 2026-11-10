//! `necro gdscan` (spec: Finders): GDScript layout checks that track state
//! across lines, read without parsing GDScript. One check per run.

use std::collections::HashSet;

use super::gdscript::{self, Kind, Source};
use crate::command::Hit;

use super::grep::read_list;
use super::tracked;
use crate::args::{self, Spec};

const USAGE: &str = "necro gdscan [--files-from <file>] [--header <n>] [--item <n>] \
                     [--write [--only-at <file>]] (spacing|inline-lambda|doc-length|export-doc|uid) [-- <pathspec>…]";

/// The checks `gdscan` knows.
const CHECKS: &[&str] = &[
    "spacing",
    "inline-lambda",
    "doc-length",
    "export-doc",
    "uid",
];

/// What `doc-length` allows when `--header` or `--item` isn't given.
const HEADER: usize = 10;
const ITEM: usize = 6;

/// Lines that may come before a file's header doc.
const PREAMBLE: &[&str] = &["@tool", "@icon", "class_name ", "extends "];

/// The `gdscan` finder.
pub fn run(args: &[String]) -> Result<Vec<Hit>, String> {
    let parsed = args::parse(
        args,
        &Spec {
            values: &["--files-from", "--only-at", "--header", "--item"],
            flags: &["--write"],
            pathspecs: true,
        },
    )
    .map_err(|e| e.with_usage(USAGE))?;
    let header = match parsed.value("--header") {
        Some(value) => lines_arg("--header", value)?,
        None => HEADER,
    };
    let item = match parsed.value("--item") {
        Some(value) => lines_arg("--item", value)?,
        None => ITEM,
    };
    let list = parsed.value("--files-from");
    let only_at = parsed.value("--only-at");
    let write = parsed.flag("--write");
    let check = args::check(&parsed.positional, CHECKS, USAGE)?;
    if (write || only_at.is_some()) && check != "spacing" {
        return Err(format!("--write goes with spacing: {USAGE}"));
    }
    let only = list.map(read_list).transpose()?;
    if check == "uid" {
        return uid(&parsed.pathspecs, only.as_ref());
    }
    let run: fn(&str, &Source, usize, usize, &mut Vec<Hit>) = match check {
        "spacing" => spacing,
        "inline-lambda" => inline_lambda,
        "doc-length" => doc_length,
        "export-doc" => export_doc,
        _ => unreachable!("args::check accepted {check}"),
    };
    let markers = ["#".to_string()];
    let mut hits = Vec::new();
    for path in gd_files(&parsed.pathspecs, only.as_ref())? {
        if let Some(src) = gdscript::scan_file(&path, &markers) {
            run(&path, &src, header, item, &mut hits);
        }
    }
    if !write {
        return Ok(hits);
    }
    let at: Option<HashSet<String>> = only_at.map(read_list).transpose()?.map(|l| {
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
fn write_spacing(hits: &[Hit], at: Option<&HashSet<String>>) -> Result<Vec<Hit>, String> {
    let mut by_file: std::collections::BTreeMap<&str, Vec<usize>> = Default::default();
    for hit in hits {
        let path = &hit.path;
        let line = hit.line;
        if at.is_some_and(|a| !a.contains(&format!("{path}:{line}"))) {
            continue;
        }
        by_file.entry(path).or_default().push(line);
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
        fixed.extend(blocks.iter().map(|line| Hit {
            path: path.to_string(),
            line: *line,
            col: 1,
            endcol: None,
            label: "fixed".to_string(),
        }));
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
fn spacing(path: &str, src: &Source, _: usize, _: usize, hits: &mut Vec<Hit>) {
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
            hits.push(Hit {
                path: path.to_string(),
                line: start + 1,
                col: 1,
                endcol: None,
                label: format!("{blank} blank line{s}; the previous block starts at line {p}"),
            });
        }
        previous = Some(start + 1);
    }
}

/// `inline-lambda`: a lambda inside brackets opened on an earlier line (gdformat corrupts these).
fn inline_lambda(path: &str, src: &Source, _: usize, _: usize, hits: &mut Vec<Hit>) {
    for (n, col, end) in lambda_spans(src) {
        hits.push(Hit {
            path: path.to_string(),
            line: n,
            col,
            endcol: Some(end),
            label: String::new(),
        });
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
fn doc_length(path: &str, src: &Source, header: usize, item: usize, hits: &mut Vec<Hit>) {
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
            hits.push(Hit {
                path: path.to_string(),
                line: line + 1,
                col: col + 1,
                endcol: None,
                label: format!("{count} lines, over {limit}"),
            });
        }
    }
}

/// `export-doc`: an `@export` with no own-line `##` comment directly above it.
fn export_doc(path: &str, src: &Source, _: usize, _: usize, hits: &mut Vec<Hit>) {
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
            hits.push(Hit {
                path: path.to_string(),
                line: n + 1,
                col,
                endcol: Some(col + 6),
                label: String::new(),
            });
        }
    }
}

/// `uid`: in a repo that tracks `.gd.uid` files (Godot 4.4 and later),
/// every tracked `.gd` has its tracked `.uid`, and every `.uid` its `.gd`.
fn uid(pathspecs: &[String], only: Option<&HashSet<String>>) -> Result<Vec<Hit>, String> {
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
    let mut hits: Vec<Hit> = gd
        .iter()
        .filter(|g| !uids.contains(*g) && wanted(g))
        .map(|g| Hit {
            path: g.to_string(),
            line: 0,
            col: 0,
            endcol: None,
            label: format!("no tracked {g}.uid"),
        })
        .chain(
            uids.iter()
                .filter(|g| !gd.contains(*g) && wanted(&format!("{g}.uid")))
                .map(|g| Hit {
                    path: format!("{g}.uid"),
                    line: 0,
                    col: 0,
                    endcol: None,
                    label: format!("no tracked {g}"),
                }),
        )
        .collect();
    hits.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(hits)
}
