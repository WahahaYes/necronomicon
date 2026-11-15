//! `necro header` (spec: Finders): a fixed header at the top of each file,
//! written behind the file's comment marker.

use std::collections::{BTreeMap, HashSet};

use super::grep::read_list;
use super::tracked;
use crate::args::{self, Spec};
use crate::command::Hit;

const USAGE: &str = "necro header [--files-from <file>] [--template <file>] \
                     [--marker <ext>=<marker>]… [--similar <percent>] \
                     [--write [--only-at <file>]] [<line>…] [-- <pathspec>…]";

/// Comment markers by extension, before `--marker`.
const MARKERS: &[(&str, &str)] = &[
    ("gd", "#"),
    ("py", "#"),
    ("sh", "#"),
    ("toml", "#"),
    ("yaml", "#"),
    ("yml", "#"),
    ("rs", "//"),
    ("c", "//"),
    ("h", "//"),
    ("cpp", "//"),
    ("hpp", "//"),
    ("cs", "//"),
    ("go", "//"),
    ("java", "//"),
    ("js", "//"),
    ("ts", "//"),
    ("gdshader", "//"),
];

/// What sits at a file's header place.
enum Found {
    /// The header, exactly.
    Header,
    /// Not the header: insert it.
    Missing,
    /// An earlier version of the header, this percent alike: replace it.
    Old(usize),
}

/// The `header` finder.
pub fn run(args: &[String]) -> Result<Vec<Hit>, String> {
    let parsed = args::parse(
        args,
        &Spec {
            values: &[
                "--files-from",
                "--only-at",
                "--template",
                "--marker",
                "--similar",
            ],
            flags: &["--write"],
            pathspecs: true,
        },
    )
    .map_err(|e| e.with_usage(USAGE))?;
    let lines = header_lines(parsed.value("--template"), &parsed.positional)?;
    let markers = markers(&parsed.values("--marker"))?;
    let similar = parsed.value("--similar").map(percent).transpose()?;
    let only = parsed.value("--files-from").map(read_list).transpose()?;
    let files: Vec<String> = tracked(&parsed.pathspecs, None)?
        .into_iter()
        .filter(|p| only.as_ref().is_none_or(|o| o.contains(p)))
        .collect();

    let mut hits = Vec::new();
    let mut plans: BTreeMap<String, (usize, Found, String)> = BTreeMap::new();
    for path in files {
        let marker = marker_for(&path, &markers)?;
        let Some(text) = read_text(&path) else {
            continue;
        };
        let expected: Vec<String> = lines
            .iter()
            .map(|l| {
                if l.is_empty() {
                    marker.to_string()
                } else {
                    format!("{marker} {l}")
                }
            })
            .collect();
        let file_lines: Vec<&str> = text.lines().map(|l| l.trim_end_matches('\r')).collect();
        let at = usize::from(file_lines.first().is_some_and(|l| l.starts_with("#!")));
        let found = judge(&file_lines, at, &expected, marker, similar);
        let label = match found {
            Found::Header => continue,
            Found::Missing => "no header".to_string(),
            Found::Old(n) => format!("old header, {n}% alike"),
        };
        hits.push(hit(&path, at + 1, label));
        plans.insert(path, (at, found, expected.join("\n")));
    }
    if !parsed.flag("--write") {
        return Ok(hits);
    }
    let only_at: Option<HashSet<String>> = parsed
        .value("--only-at")
        .map(read_list)
        .transpose()?
        .map(|l| {
            l.iter()
                .map(|h| h.splitn(3, ':').take(2).collect::<Vec<_>>().join(":"))
                .collect()
        });
    let mut fixed = Vec::new();
    for (path, (at, found, header)) in plans {
        if only_at
            .as_ref()
            .is_some_and(|a| !a.contains(&format!("{path}:{}", at + 1)))
        {
            continue;
        }
        write(&path, at, &found, &header)?;
        fixed.push(hit(&path, at + 1, "fixed".to_string()));
    }
    Ok(fixed)
}

/// The header's lines, from `--template` or the line words: exactly one.
fn header_lines(template: Option<&str>, words: &[String]) -> Result<Vec<String>, String> {
    let mut lines = match (template, words.is_empty()) {
        (Some(_), false) => {
            return Err(format!(
                "give the header as --template <file> or as lines, not both: {USAGE}"
            ));
        }
        (Some(file), true) => std::fs::read_to_string(file)
            .map_err(|e| format!("can't read {file}: {e}"))?
            .lines()
            .map(|l| l.trim_end_matches('\r').to_string())
            .collect(),
        (None, false) => words.to_vec(),
        (None, true) => Vec::new(),
    };
    while lines.last().is_some_and(|l| l.trim().is_empty()) {
        lines.pop();
    }
    if lines.is_empty() {
        return Err(format!(
            "give the header as --template <file> or as lines: {USAGE}"
        ));
    }
    Ok(lines)
}

/// The marker table with each `--marker <ext>=<marker>` applied.
fn markers(given: &[&str]) -> Result<BTreeMap<String, String>, String> {
    let mut table: BTreeMap<String, String> = MARKERS
        .iter()
        .map(|(e, m)| (e.to_string(), m.to_string()))
        .collect();
    for pair in given {
        match pair.split_once('=') {
            Some((ext, marker)) if !ext.is_empty() && !marker.is_empty() => {
                table.insert(ext.trim_start_matches('.').to_string(), marker.to_string());
            }
            _ => return Err(format!("--marker takes <ext>=<marker>, got {pair}")),
        }
    }
    Ok(table)
}

/// `--similar`'s value: a whole number from 1 to 100.
fn percent(value: &str) -> Result<usize, String> {
    match value.parse::<usize>() {
        Ok(n) if (1..=100).contains(&n) => Ok(n),
        _ => Err(format!(
            "--similar takes a whole number from 1 to 100, got {value}"
        )),
    }
}

/// The marker for `path`'s extension.
fn marker_for<'a>(path: &str, table: &'a BTreeMap<String, String>) -> Result<&'a str, String> {
    let name = path.rsplit('/').next().unwrap_or(path);
    let ext = name.rsplit_once('.').map(|(_, e)| e).unwrap_or("");
    table.get(ext).map(String::as_str).ok_or_else(|| {
        let ext = if ext.is_empty() { "<ext>" } else { ext };
        format!("no comment marker for {path}; give --marker {ext}=<marker>")
    })
}

/// A file's text, or `None` for one that can't be read as UTF-8.
fn read_text(path: &str) -> Option<String> {
    String::from_utf8(std::fs::read(path).ok()?).ok()
}

/// What sits at line `at` (0-based): the header, an old version of it, or neither.
fn judge(
    lines: &[&str],
    at: usize,
    expected: &[String],
    marker: &str,
    similar: Option<usize>,
) -> Found {
    let block: Vec<&str> = lines
        .iter()
        .skip(at)
        .take(expected.len())
        .copied()
        .collect();
    if block.len() == expected.len() && block.iter().zip(expected).all(|(a, b)| *a == b) {
        return Found::Header;
    }
    let Some(least) = similar else {
        return Found::Missing;
    };
    if block.len() < expected.len() || !block.iter().all(|l| l.starts_with(marker)) {
        return Found::Missing;
    }
    let alike = alike(&block.join("\n"), &expected.join("\n"));
    if alike >= least {
        Found::Old(alike)
    } else {
        Found::Missing
    }
}

/// How alike two texts are, in whole percent rounded down: one minus the
/// character edit distance over the longer length.
fn alike(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let longest = a.len().max(b.len());
    if longest == 0 {
        return 100;
    }
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != cb);
            let next = (row[j + 1] + 1).min(row[j] + 1).min(prev + cost);
            prev = row[j + 1];
            row[j + 1] = next;
        }
    }
    let distance = row[b.len()];
    (longest - distance) * 100 / longest
}

/// Put `header` at line `at` of `path`, replacing an old header, with a blank
/// line after it unless one follows, in the file's line endings.
fn write(path: &str, at: usize, found: &Found, header: &str) -> Result<(), String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("can't read {path}: {e}"))?;
    let eol = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut lines: Vec<String> = text.split_inclusive('\n').map(String::from).collect();
    if let Some(last) = lines.last_mut()
        && !last.ends_with('\n')
    {
        last.push_str(eol);
    }
    let header: Vec<String> = header.split('\n').map(|l| format!("{l}{eol}")).collect();
    let count = header.len();
    let at = at.min(lines.len());
    if let Found::Old(_) = found {
        lines.drain(at..(at + count).min(lines.len()));
    }
    let rest_blank = lines.get(at).is_none_or(|l| l.trim().is_empty());
    let mut insert = header;
    if !rest_blank {
        insert.push(eol.to_string());
    }
    lines.splice(at..at, insert);
    std::fs::write(path, lines.concat()).map_err(|e| format!("can't write {path}: {e}"))
}

fn hit(path: &str, line: usize, label: String) -> Hit {
    Hit {
        path: path.to_string(),
        line,
        col: 1,
        endcol: None,
        label,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alike_rounds_down() {
        assert_eq!(alike("abc", "abc"), 100);
        assert_eq!(alike("abc", "abd"), 66);
        assert_eq!(alike("", ""), 100);
    }

    #[test]
    fn write_keeps_crlf() {
        let dir = std::env::temp_dir().join(format!("necro-header-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("a.gd");
        std::fs::write(&path, "extends Node\r\n").unwrap();
        let p = path.to_str().unwrap();
        write(p, 0, &Found::Missing, "# H").unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "# H\r\n\r\nextends Node\r\n"
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
