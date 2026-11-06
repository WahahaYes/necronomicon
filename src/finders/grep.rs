//! `necro grep` (spec: Finders): every regex match in the tracked files.
//!
//! `git grep -F` picks the files holding fixed text every match must contain;
//! necro matches them on all cores with `fancy_regex` (lookaround works).

use std::sync::atomic::{AtomicUsize, Ordering};

use fancy_regex::Regex;
use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use regex_syntax::hir::literal::{ExtractKind, Extractor};

use super::{regex_error, split_pathspecs, text_lines, tracked};

const USAGE: &str = "necro grep [--skip-comments <marker>]… [--glob <glob>]… \
     [--files-from <file>] <regex> [-- <pathspec>…]";

/// Most strings handed to `git grep -F`; past it, every file is searched.
const MAX_LITERALS: usize = 64;

/// Files per thread: a small search stays on few threads, since rules
/// already run side by side.
const FILES_PER_THREAD: usize = 64;

/// The `grep` finder: every regex match in the tracked files.
pub fn run(args: &[String]) -> Result<Vec<String>, String> {
    let (opts, pathspecs) = split_pathspecs(args);
    let mut markers: Vec<String> = Vec::new();
    let mut globs: Vec<String> = Vec::new();
    let mut lists: Vec<String> = Vec::new();
    let mut pattern: Option<&String> = None;
    let mut it = opts.iter();
    while let Some(arg) = it.next() {
        if let Some((name, list)) = [
            ("--skip-comments", &mut markers),
            ("--glob", &mut globs),
            ("--files-from", &mut lists),
        ]
        .into_iter()
        .find(|(name, _)| arg == name || arg.starts_with(&format!("{name}=")))
        {
            let value = match arg.strip_prefix(&format!("{name}=")) {
                Some(v) => v.to_string(),
                None => it
                    .next()
                    .ok_or(format!("{name} needs a value: {USAGE}"))?
                    .clone(),
            };
            list.push(value);
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
    let re = Regex::new(pattern)
        .map_err(|e| format!("`{pattern}` isn't a valid regex ({})", regex_error(&e)))?;
    let globs = Globs::new(&globs)?;
    let only = match lists.as_slice() {
        [] => None,
        [list] => Some(read_list(list)?),
        _ => return Err(format!("give --files-from once: {USAGE}")),
    };
    let mut specs = pathspecs.to_vec();
    specs.extend(globs.pushdown(pathspecs.is_empty()));
    let files: Vec<String> = tracked(&specs, required_text(pattern).as_deref())?
        .into_iter()
        .filter(|path| globs.keeps(path))
        .filter(|path| only.as_ref().is_none_or(|o| o.contains(path)))
        .collect();
    search(&files, &re, &markers)
}

/// `--files-from`: paths relative to the working folder, one per line.
pub(super) fn read_list(list: &str) -> Result<std::collections::HashSet<String>, String> {
    let text = std::fs::read_to_string(list).map_err(|e| format!("can't read {list}: {e}"))?;
    Ok(text
        .lines()
        .map(|l| l.strip_prefix("./").unwrap_or(l).to_string())
        .filter(|l| !l.is_empty())
        .collect())
}

/// The `--glob` filters, as prek reads globs: matched against the whole
/// path, `*` crossing `/`; a leading `!` excludes.
struct Globs {
    include: Vec<String>,
    include_set: Option<GlobSet>,
    exclude_set: GlobSet,
}

impl Globs {
    fn new(globs: &[String]) -> Result<Globs, String> {
        let mut include = Vec::new();
        let (mut inc, mut exc) = (GlobSetBuilder::new(), GlobSetBuilder::new());
        for raw in globs {
            let (negated, body) = match raw.strip_prefix('!') {
                Some(body) => (true, body),
                None => (false, raw.as_str()),
            };
            let body = body.strip_prefix("./").unwrap_or(body);
            let glob = GlobBuilder::new(body)
                .build()
                .map_err(|e| format!("`{raw}` isn't a valid glob ({})", e.kind()))?;
            if negated {
                exc.add(glob);
            } else {
                inc.add(glob);
                include.push(body.to_string());
            }
        }
        let built = |b: GlobSetBuilder| b.build().map_err(|e| e.to_string());
        Ok(Globs {
            include_set: if include.is_empty() {
                None
            } else {
                Some(built(inc)?)
            },
            exclude_set: built(exc)?,
            include,
        })
    }

    fn keeps(&self, path: &str) -> bool {
        self.include_set.as_ref().is_none_or(|s| s.is_match(path))
            && !self.exclude_set.is_match(path)
    }

    /// The includes as git pathspecs, so `git grep` skips what they rule
    /// out; `keeps` still decides. Only when there are no pathspecs (git ORs
    /// them) and every include is `*.<suffix>`, which `:(glob)**/*.<suffix>`
    /// matches exactly; git's globs stop `*` at `/`, so others would miss
    /// files.
    fn pushdown(&self, no_pathspecs: bool) -> Vec<String> {
        let suffix_only = |g: &String| {
            g.strip_prefix('*')
                .is_some_and(|rest| !rest.contains(['*', '?', '[', ']', '{', '}', '\\', '/']))
        };
        if no_pathspecs && !self.include.is_empty() && self.include.iter().all(suffix_only) {
            self.include
                .iter()
                .map(|g| format!(":(glob)**/{g}"))
                .collect()
        } else {
            Vec::new()
        }
    }
}

/// Strings one of which every match contains (the regex's prefix or suffix
/// literals, whichever set's shortest string is longer), for `git grep -F` to
/// pick the files; `None` when no set is usable. A newline (git splits on it)
/// or U+FFFD (lossy UTF-8 reading invents it) spoils a set.
fn required_text(pattern: &str) -> Option<Vec<String>> {
    let hir = regex_syntax::parse(pattern).ok()?;
    [ExtractKind::Prefix, ExtractKind::Suffix]
        .into_iter()
        .filter_map(|kind| {
            let seq = Extractor::new().kind(kind).extract(&hir);
            let literals = seq.literals()?;
            if literals.is_empty() || literals.len() > MAX_LITERALS {
                return None;
            }
            let mut texts: Vec<String> = literals
                .iter()
                .map(|l| String::from_utf8(l.as_bytes().to_vec()).ok())
                .collect::<Option<_>>()?;
            if texts
                .iter()
                .any(|t| t.is_empty() || t.contains(['\n', '\u{FFFD}']))
            {
                return None;
            }
            texts.sort();
            texts.dedup();
            Some(texts)
        })
        .max_by_key(|texts| texts.iter().map(String::len).min())
}

/// Every hit in the files, on all cores, in the files' order.
fn search(files: &[String], re: &Regex, markers: &[String]) -> Result<Vec<String>, String> {
    let threads = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .min(files.len().div_ceil(FILES_PER_THREAD))
        .max(1);
    let next = AtomicUsize::new(0);
    let found: Vec<(usize, Vec<String>)> = std::thread::scope(|s| {
        let workers: Vec<_> = (0..threads)
            .map(|_| {
                s.spawn(|| {
                    let mut out = Vec::new();
                    loop {
                        let i = next.fetch_add(1, Ordering::Relaxed);
                        let Some(path) = files.get(i) else { break };
                        let hits = file_hits(path, re, markers)?;
                        if !hits.is_empty() {
                            out.push((i, hits));
                        }
                    }
                    Ok(out)
                })
            })
            .collect();
        workers
            .into_iter()
            .map(|w| w.join().unwrap_or_else(|e| std::panic::resume_unwind(e)))
            .collect::<Result<Vec<Vec<_>>, String>>()
            .map(|parts| parts.into_iter().flatten().collect())
    })?;
    let mut found = found;
    found.sort_by_key(|(i, _)| *i);
    Ok(found.into_iter().flat_map(|(_, hits)| hits).collect())
}

/// One file's hits, `path:line:col-endcol: <match>`; an error when the
/// regex gives up on a line (fancy_regex's backtracking limit).
fn file_hits(path: &str, re: &Regex, markers: &[String]) -> Result<Vec<String>, String> {
    let Some(lines) = text_lines(path) else {
        return Ok(Vec::new());
    };
    let mut hits = Vec::new();
    for (n, line) in lines.iter().enumerate() {
        let code = &line[..code_end(line, markers)];
        for m in re.find_iter(code) {
            let m = m.map_err(|e| format!("{path}:{}: the regex gave up ({e})", n + 1))?;
            if m.as_str().is_empty() {
                continue;
            }
            let col = code[..m.start()].chars().count() + 1;
            let end = col + m.as_str().chars().count() - 1;
            hits.push(format!("{path}:{}:{col}-{end}: {}", n + 1, m.as_str()));
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
