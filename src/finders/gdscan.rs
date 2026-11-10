//! `necro gdscan` (spec: Finders): GDScript layout checks that track state
//! across lines, read without parsing GDScript. One check per run.

use std::collections::HashSet;

use crate::command::Hit;
use regex::Regex;

use super::grep::read_list;
use super::{split_pathspecs, tracked};

const USAGE: &str = "necro gdscan [--files-from <file>] [--header <n>] [--item <n>] \
                     [--write [--only-at <file>]] (spacing|inline-lambda|doc-length|export-doc|uid) [-- <pathspec>…]";

/// What `doc-length` allows when `--header` or `--item` isn't given.
const HEADER: usize = 10;
const ITEM: usize = 6;

/// Lines that may come before a file's header doc.
const PREAMBLE: &[&str] = &["@tool", "@icon", "class_name ", "extends "];

/// One comment: its line and column (0-based, in chars), marker, and
/// whether it's the first thing on its line.
struct Comment {
    line: usize,
    col: usize,
    doc: bool,
    own_line: bool,
}

/// A file's lines, the same lines with comments blanked (strings kept), and its comments.
struct Source {
    lines: Vec<String>,
    code: Vec<String>,
    comments: Vec<Comment>,
    /// Lines that start inside a triple-quoted string: text, not code.
    in_string: Vec<bool>,
}

/// The `gdscan` finder.
pub fn run(args: &[String]) -> Result<Vec<Hit>, String> {
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
    let run: fn(&str, &Source, usize, usize, &mut Vec<Hit>) = match check {
        "spacing" => spacing,
        "inline-lambda" => inline_lambda,
        "doc-length" => doc_length,
        "export-doc" => export_doc,
        other => return Err(format!("unknown check {other}: {USAGE}")),
    };
    let mut hits = Vec::new();
    for path in gd_files(pathspecs, only.as_ref())? {
        if let Some(src) = scan(&path) {
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

/// Read a file into lines, code and comments; `None` if it can't be read.
fn scan(path: &str) -> Option<Source> {
    let bytes = std::fs::read(path).ok()?;
    let text = String::from_utf8_lossy(&bytes);
    let mut lines: Vec<String> = text
        .split('\n')
        .map(|l| l.strip_suffix('\r').unwrap_or(l).to_string())
        .collect();
    if lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    let mut code = Vec::with_capacity(lines.len());
    let mut comments = Vec::new();
    let mut quote: Option<&str> = None;
    let mut in_string = Vec::with_capacity(lines.len());
    for (n, line) in lines.iter().enumerate() {
        in_string.push(quote.is_some());
        let chars: Vec<char> = line.chars().collect();
        let mut out = chars.clone();
        let first = chars.iter().take_while(|c| c.is_whitespace()).count();
        let at = |i: usize, s: &str| {
            s.chars()
                .enumerate()
                .all(|(k, c)| chars.get(i + k) == Some(&c))
        };
        let mut i = 0;
        while i < chars.len() {
            if let Some(q) = quote {
                if chars[i] == '\\' {
                    i += 2;
                } else if at(i, q) {
                    i += q.chars().count();
                    quote = None;
                } else {
                    i += 1;
                }
                continue;
            }
            if chars[i] == '#' {
                comments.push(Comment {
                    line: n,
                    col: i,
                    doc: at(i, "##"),
                    own_line: i == first,
                });
                for c in &mut out[i..] {
                    *c = ' ';
                }
                break;
            }
            quote = ["\"\"\"", "\"", "'"].into_iter().find(|q| at(i, q));
            i += quote.map_or(1, |q| q.chars().count());
        }
        // Only a triple-quoted string runs on past its line.
        if quote != Some("\"\"\"") {
            quote = None;
        }
        code.push(out.into_iter().collect());
    }
    Some(Source {
        lines,
        code,
        comments,
        in_string,
    })
}

/// The bracket depth at the start of each line, outside strings and comments.
fn depths(lines: &[String]) -> Vec<usize> {
    let mut depth = 0usize;
    let mut out = Vec::with_capacity(lines.len());
    for raw in lines {
        out.push(depth);
        let mut quote: Option<char> = None;
        let mut chars = raw.chars();
        while let Some(c) = chars.next() {
            match quote {
                Some(_) if c == '\\' => {
                    chars.next();
                }
                Some(q) if c == q => quote = None,
                Some(_) => {}
                None if c == '\'' || c == '"' => quote = Some(c),
                None if c == '#' => break,
                None if "([{".contains(c) => depth += 1,
                None if ")]}".contains(c) => depth = depth.saturating_sub(1),
                None => {}
            }
        }
    }
    out
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
    for (i, depth) in depths(lines).into_iter().enumerate() {
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
    let Ok(re) = Regex::new(r"^func\s*\(") else {
        return vec![];
    };
    let mut out = Vec::new();
    for (n, (raw, start)) in src.lines.iter().zip(depths(&src.lines)).enumerate() {
        if src.in_string[n] {
            continue;
        }
        let mut depth = start;
        let mut quote: Option<char> = None;
        let mut prev_word = false;
        let mut chars = raw.char_indices().enumerate();
        while let Some((col, (at, c))) = chars.next() {
            match quote {
                Some(_) if c == '\\' => {
                    chars.next();
                }
                Some(q) if c == q => quote = None,
                Some(_) => {}
                None if c == '\'' || c == '"' => quote = Some(c),
                None if c == '#' => break,
                None if "([{".contains(c) => depth += 1,
                None if ")]}".contains(c) => depth = depth.saturating_sub(1),
                None if depth > 0 && !prev_word => {
                    if let Some(m) = re.find(&raw[at..]) {
                        out.push((n + 1, col + 1, col + m.as_str().chars().count()));
                    }
                }
                None => {}
            }
            prev_word = c.is_alphanumeric() || c == '_';
        }
    }
    out
}

/// Whether a file holds a lambda that gdformat would corrupt.
pub(super) fn has_inline_lambda(path: &str) -> bool {
    scan(path).is_some_and(|src| !lambda_spans(&src).is_empty())
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
