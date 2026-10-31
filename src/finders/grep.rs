//! `necro grep` (spec: Finders): every regex match in the tracked files.

use regex::Regex;

use super::{split_pathspecs, text_lines, tracked};

const USAGE: &str = "necro grep [--skip-comments <marker>]… <regex> [-- <pathspec>…]";

/// The `grep` finder: every regex match in the tracked files.
pub fn run(args: &[String]) -> Result<Vec<String>, String> {
    let (opts, pathspecs) = split_pathspecs(args);
    let mut markers: Vec<String> = Vec::new();
    let mut pattern: Option<&String> = None;
    let mut it = opts.iter();
    while let Some(arg) = it.next() {
        if let Some(m) = arg.strip_prefix("--skip-comments=") {
            markers.push(m.to_string());
        } else if arg == "--skip-comments" {
            let m = it
                .next()
                .ok_or(format!("--skip-comments needs a marker: {USAGE}"))?;
            markers.push(m.clone());
        } else if arg.starts_with('-') && arg.len() > 1 && pattern.is_none() {
            return Err(format!("unknown option {arg}: {USAGE}"));
        } else if pattern.is_none() {
            pattern = Some(arg);
        } else {
            return Err(format!(
                "unexpected argument {arg}; pathspecs go after --: {USAGE}"
            ));
        }
    }
    if markers.iter().any(String::is_empty) {
        return Err("--skip-comments needs a non-empty marker".into());
    }
    let pattern = pattern.ok_or(format!("give a regex: {USAGE}"))?;
    let re = Regex::new(pattern).map_err(|e| {
        let why = e.to_string();
        let last = why.lines().last().unwrap_or("").trim();
        let last = last.strip_prefix("error: ").unwrap_or(last);
        format!("`{pattern}` isn't a valid regex ({last})")
    })?;
    let mut hits = Vec::new();
    for path in tracked(pathspecs)? {
        let Some(lines) = text_lines(&path) else {
            continue;
        };
        for (n, line) in lines.iter().enumerate() {
            let code = &line[..code_end(line, &markers)];
            for m in re.find_iter(code).filter(|m| !m.is_empty()) {
                let col = code[..m.start()].chars().count() + 1;
                let end = col + m.as_str().chars().count() - 1;
                hits.push(format!("{path}:{}:{col}-{end}: {}", n + 1, m.as_str()));
            }
        }
    }
    Ok(hits)
}

/// The byte offset where a line's comment starts: the first marker outside a
/// `"…"` or `'…'` string, or the line's end.
fn code_end(line: &str, markers: &[String]) -> usize {
    if markers.is_empty() {
        return line.len();
    }
    let mut quote: Option<char> = None;
    let mut chars = line.char_indices();
    while let Some((i, c)) = chars.next() {
        match quote {
            Some(_) if c == '\\' => {
                chars.next();
            }
            Some(q) if c == q => quote = None,
            Some(_) => {}
            None if c == '"' || c == '\'' => quote = Some(c),
            None if markers.iter().any(|m| line[i..].starts_with(m.as_str())) => return i,
            None => {}
        }
    }
    line.len()
}
