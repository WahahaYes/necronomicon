//! `necro rust-docs` (spec: Finders): doc comments in tracked `.rs` files,
//! read line by line without parsing Rust. One check per run: `missing`,
//! `module` or `length`.

use std::path::Path;

use super::grep::read_list;
use super::{split_pathspecs, text_lines, tracked};

const USAGE: &str = "necro rust-docs [--files-from <file>] [--header <n>] [--item <n>] \
                     (missing|module|length) [-- <pathspec>…]";

/// What `length` allows when `--header` or `--item` isn't given.
const HEADER: usize = 10;
const ITEM: usize = 6;

/// The item kinds `missing` and `length` know, after `pub` and qualifiers.
const KINDS: &[&str] = &[
    "fn", "struct", "enum", "trait", "type", "const", "static", "mod",
];
const QUALIFIERS: &[&str] = &["unsafe", "async", "const"];
/// Items whose doc gets the header limit.
const TYPE_KINDS: &[&str] = &["struct", "enum", "trait", "union", "impl"];

/// The `rust-docs` finder.
pub fn run(args: &[String]) -> Result<Vec<String>, String> {
    let (opts, pathspecs) = split_pathspecs(args);
    let (mut header, mut item) = (HEADER, ITEM);
    let mut list: Option<String> = None;
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
            "--header" => header = count(flag, &value()?)?,
            "--item" => item = count(flag, &value()?)?,
            f if f.starts_with('-') => return Err(format!("unknown option {f}: {USAGE}")),
            c if check.is_none() => check = Some(c),
            other => return Err(format!("unexpected argument {other}: {USAGE}")),
        }
    }
    let check = check.ok_or(format!("name a check: {USAGE}"))?;
    if !["missing", "module", "length"].contains(&check) {
        return Err(format!("unknown check {check}: {USAGE}"));
    }
    let only = list.as_deref().map(read_list).transpose()?;
    let mut specs = pathspecs.to_vec();
    if specs.is_empty() {
        specs.push("*.rs".into());
    }
    let mut hits = Vec::new();
    for path in tracked(&specs, None)? {
        if !path.ends_with(".rs") || only.as_ref().is_some_and(|o| !o.contains(&path)) {
            continue;
        }
        let Some(lines) = text_lines(&path) else {
            continue;
        };
        match check {
            "missing" => missing(&path, &lines, &mut hits),
            "module" => module(&path, &lines, &mut hits),
            _ => length(&path, &lines, header, item, &mut hits),
        }
    }
    Ok(hits)
}

/// A limit: a whole number of lines, at least 1.
fn count(flag: &str, value: &str) -> Result<usize, String> {
    match value.parse::<usize>() {
        Ok(n) if n > 0 => Ok(n),
        _ => Err(format!("{flag} takes a whole number of lines, got {value}")),
    }
}

/// The line without its `//` comment, strings kept; `'` opens a char only
/// when it closes within two characters (else it's a lifetime).
fn code_part(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::new();
    let mut quote: Option<char> = None;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if let Some(q) = quote {
            out.push(c);
            if c == '\\' {
                if let Some(next) = chars.get(i + 1) {
                    out.push(*next);
                    i += 1;
                }
            } else if c == q {
                quote = None;
            }
        } else if c == '"' {
            quote = Some(c);
            out.push(c);
        } else if c == '\'' {
            let closes = chars.get(i + 2) == Some(&'\'')
                || (chars.get(i + 1) == Some(&'\\') && chars.get(i + 3) == Some(&'\''));
            if closes {
                quote = Some(c);
            }
            out.push(c);
        } else if c == '/' && chars.get(i + 1) == Some(&'/') {
            break;
        } else {
            out.push(c);
        }
        i += 1;
    }
    out
}

/// Bracket depth change on a comment-stripped line.
fn depth_delta(code: &str) -> i64 {
    code.chars()
        .map(|c| match c {
            '{' | '[' | '(' => 1,
            '}' | ']' | ')' => -1,
            _ => 0,
        })
        .sum()
}

/// Which lines belong to an attribute (`#[…]`, `#![…]`), multi-line ones included.
fn attribute_lines(lines: &[String]) -> Vec<bool> {
    let mut out = vec![false; lines.len()];
    let mut open = 0;
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim_start();
        if open > 0 || t.starts_with("#[") || t.starts_with("#![") {
            out[i] = true;
            open = (open + depth_delta(&code_part(line))).max(0);
        }
    }
    out
}

/// A public item on this line: (column of `pub`, kind, name).
fn public_item(line: &str) -> Option<(usize, &str, &str)> {
    let rest = line.trim_start();
    if !rest.starts_with("pub ") && !rest.starts_with("pub\t") {
        return None;
    }
    let col = line[..line.len() - rest.len()].chars().count() + 1;
    let mut words = rest[3..]
        .split(|c: char| c.is_whitespace() || "(<{:;=".contains(c))
        .filter(|w| !w.is_empty());
    let mut kind = words.next()?;
    // Qualifiers come first; `const` is one before `fn`, else the item itself.
    while QUALIFIERS.contains(&kind) {
        match words.next() {
            Some(w) if KINDS.contains(&w) || QUALIFIERS.contains(&w) => kind = w,
            Some(w) if kind == "const" => return Some((col, "const", w)),
            _ => return None,
        }
    }
    if !KINDS.contains(&kind) {
        return None;
    }
    let name = match words.next() {
        Some("mut") => words.next().unwrap_or(""),
        other => other.unwrap_or(""),
    };
    Some((col, kind, name))
}

/// `missing`: public items with no `///` above them.
fn missing(path: &str, lines: &[String], hits: &mut Vec<String>) {
    let attr = attribute_lines(lines);
    // (depth inside the module's braces, whether it's a test module)
    let mut modules: Vec<(i64, bool)> = Vec::new();
    let mut depth: i64 = 0;
    let mut pending_test = false;
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim();
        if let Some((col, kind, name)) = public_item(line).filter(|_| !attr[i]) {
            let mut j = i;
            let mut guarded = modules.iter().any(|(_, test)| *test);
            while j > 0 && (lines[j - 1].trim().is_empty() || attr[j - 1]) {
                j -= 1;
                let a = lines[j].trim();
                if a.contains("cfg(test)") || a == "#[test]" {
                    guarded = true;
                }
            }
            let documented = j > 0 && is_outer_doc(&lines[j - 1]);
            let inner =
                kind == "mod" && t.trim_end().ends_with(';') && module_file_has_doc(path, name);
            if !guarded && !documented && !inner {
                let label = if name.is_empty() {
                    kind.to_string()
                } else {
                    format!("{kind} {name}")
                };
                hits.push(format!("{path}:{}:{col}: {label}", i + 1));
            }
        }
        let code = code_part(line);
        if attr[i] {
            if t.contains("cfg(test)") {
                pending_test = true;
            }
        } else if !t.is_empty() {
            if opens_module(&code) {
                modules.push((depth + depth_delta(&code), pending_test));
            }
            pending_test = false;
        }
        depth += depth_delta(&code);
        while modules.last().is_some_and(|(d, _)| depth < *d) {
            modules.pop();
        }
    }
}

/// Whether the code opens a module body: `mod name {` (any visibility).
fn opens_module(code: &str) -> bool {
    let words: Vec<&str> = code
        .split(|c: char| c.is_whitespace() || c == '{')
        .filter(|w| !w.is_empty())
        .collect();
    let at = words.iter().position(|w| *w == "mod");
    at.is_some_and(|k| {
        words[..k].iter().all(|w| w.starts_with("pub")) && code.trim_end().ends_with('{')
    })
}

/// A `///` line, not `////`.
fn is_outer_doc(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("///") && !t.starts_with("////")
}

/// A `///` or `//!` line, not `////`.
fn is_doc(line: &str) -> bool {
    let t = line.trim_start();
    is_outer_doc(line) || t.starts_with("//!")
}

/// Whether `pub mod <name>;` declared in `path` lives in a file that opens with `//!`.
fn module_file_has_doc(path: &str, name: &str) -> bool {
    let file = Path::new(path);
    let dir = file.parent().unwrap_or(Path::new(""));
    let stem = file.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    // main.rs, lib.rs and mod.rs own their folder; any other file owns a folder named after it.
    let base = if ["main", "lib", "mod"].contains(&stem) {
        dir.to_path_buf()
    } else {
        dir.join(stem)
    };
    [
        base.join(format!("{name}.rs")),
        base.join(name).join("mod.rs"),
    ]
    .iter()
    .filter_map(|p| p.to_str().and_then(text_lines))
    .any(|lines| first_content(&lines).is_some_and(|l| l.trim_start().starts_with("//!")))
}

/// The first line that isn't blank or an inner attribute.
fn first_content(lines: &[String]) -> Option<&String> {
    let attr = attribute_lines(lines);
    lines
        .iter()
        .enumerate()
        .find(|(i, l)| {
            !l.trim().is_empty()
                && !(attr[*i] && l.trim_start().starts_with("#!"))
                && !inner_attr_tail(lines, &attr, *i)
        })
        .map(|(_, l)| l)
}

/// Whether line `i` continues an inner attribute begun on an earlier line.
fn inner_attr_tail(lines: &[String], attr: &[bool], i: usize) -> bool {
    if !attr[i] {
        return false;
    }
    let mut k = i;
    while k > 0 && attr[k - 1] && !lines[k].trim_start().starts_with('#') {
        k -= 1;
    }
    lines[k].trim_start().starts_with("#!")
}

/// `module`: a file that doesn't open with `//!`.
fn module(path: &str, lines: &[String], hits: &mut Vec<String>) {
    if let Some(first) = first_content(lines)
        && !first.trim_start().starts_with("//!")
    {
        hits.push(format!("{path}:1:1: no //! module doc"));
    }
}

/// `length`: doc runs over their limit.
fn length(path: &str, lines: &[String], header: usize, item: usize, hits: &mut Vec<String>) {
    let attr = attribute_lines(lines);
    let mut i = 0;
    while i < lines.len() {
        if !is_doc(&lines[i]) {
            i += 1;
            continue;
        }
        let start = i;
        while i < lines.len() && is_doc(&lines[i]) {
            i += 1;
        }
        let run = &lines[start..i];
        let mut next = i;
        while next < lines.len() && attr[next] {
            next += 1;
        }
        let is_header = run.iter().any(|l| l.trim_start().starts_with("//!"))
            || lines.get(next).is_some_and(|l| declares_type(l));
        let limit = if is_header { header } else { item };
        if run.len() > limit {
            hits.push(format!(
                "{path}:{}:1: doc block is {} lines (limit {limit})",
                start + 1,
                run.len()
            ));
        }
    }
}

/// Whether the line declares a struct, enum, trait, union or impl, any visibility.
fn declares_type(line: &str) -> bool {
    line.split(|c: char| c.is_whitespace() || c == '<' || c == '{')
        .filter(|w| !w.is_empty())
        .find(|w| !w.starts_with("pub") && *w != "unsafe")
        .is_some_and(|w| TYPE_KINDS.contains(&w))
}
