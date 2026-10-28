//! The checks the engine runs itself: `regex` and `engine.*` (spec: Checks,
//! Built-in checks, Allows). Each takes a scanned file and returns raw hits.

use std::collections::BTreeMap;

use regex::Regex;

use crate::config::ParamValue;
use crate::lang::{self, Lang, Source};

/// A hit before it meets its rule: 1-based line (0 = whole file) and column.
#[derive(Clone, Debug)]
pub struct RawHit {
    pub line: usize,
    pub col: usize,
    pub length: usize,
    pub values: BTreeMap<String, Value>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Str(String),
    Number(f64),
}

/// Code-point column (1-based) of byte offset `at` in `line`.
fn col(line: &str, at: usize) -> usize {
    line[..at].chars().count() + 1
}

fn captures(re: &Regex, caps: &regex::Captures) -> BTreeMap<String, Value> {
    re.capture_names()
        .flatten()
        .map(|n| {
            (
                n.to_string(),
                Value::Str(caps.name(n).map_or("", |m| m.as_str()).to_string()),
            )
        })
        .collect()
}

pub fn regex(src: &Source, re: &Regex, raw_line: bool) -> Vec<RawHit> {
    let lines = if raw_line { &src.lines } else { &src.code };
    let mut hits = Vec::new();
    for (n, line) in lines.iter().enumerate() {
        for caps in re.captures_iter(line) {
            let m = caps.get(0).expect("group 0");
            hits.push(RawHit {
                line: n + 1,
                col: col(line, m.start()),
                length: m.as_str().chars().count(),
                values: captures(re, &caps),
            });
        }
    }
    hits
}

fn param_str<'a>(params: &'a BTreeMap<String, ParamValue>, key: &str) -> &'a str {
    match params.get(key) {
        Some(ParamValue::Str(s)) => s,
        _ => "",
    }
}

fn param_list<'a>(params: &'a BTreeMap<String, ParamValue>, key: &str) -> &'a [String] {
    match params.get(key) {
        Some(ParamValue::StrList(l)) => l,
        _ => &[],
    }
}

fn param_num(params: &BTreeMap<String, ParamValue>, key: &str) -> f64 {
    match params.get(key) {
        Some(ParamValue::Number(n)) => *n,
        _ => 0.0,
    }
}

/// `engine.require`: each pattern must match somewhere (or on line 1 with `at: start`).
pub fn require(src: &Source, params: &BTreeMap<String, ParamValue>) -> Vec<RawHit> {
    let lines = if param_str(params, "in") == "line" {
        &src.lines
    } else {
        &src.code
    };
    let start = param_str(params, "at") == "start";
    let mut hits = Vec::new();
    for pattern in param_list(params, "patterns") {
        let Ok(re) = Regex::new(pattern) else {
            continue;
        };
        let found = if start {
            lines
                .first()
                .is_some_and(|l| re.find(l).is_some_and(|m| m.start() == 0))
        } else {
            lines.iter().any(|l| re.is_match(l))
        };
        if !found {
            hits.push(RawHit {
                line: 0,
                col: 1,
                length: 0,
                values: BTreeMap::from([("pattern".to_string(), Value::Str(pattern.clone()))]),
            });
        }
    }
    hits
}

/// `engine.doc-above`: a target line needs a comment on the line above,
/// looking past lines that match `skip`.
pub fn doc_above(src: &Source, lang: &Lang, params: &BTreeMap<String, ParamValue>) -> Vec<RawHit> {
    let Ok(target) = Regex::new(param_str(params, "target")) else {
        return vec![];
    };
    let skip = Some(param_str(params, "skip"))
        .filter(|s| !s.is_empty())
        .and_then(|s| Regex::new(s).ok());
    let markers = param_list(params, "markers");
    let wanted = |m: &str| {
        if markers.is_empty() {
            lang.line.iter().any(|l| l == m)
        } else {
            markers.iter().any(|k| k == m)
        }
    };
    let mut hits = Vec::new();
    for (n, line) in src.code.iter().enumerate() {
        let Some(caps) = target.captures(line) else {
            continue;
        };
        let mut j = n;
        while j > 0 && skip.as_ref().is_some_and(|s| s.is_match(&src.lines[j - 1])) {
            j -= 1;
        }
        let documented = j > 0
            && src
                .comments
                .iter()
                .any(|c| c.line == j - 1 && c.own_line && c.block.is_none() && wanted(&c.marker));
        if !documented {
            let m = caps.get(0).expect("group 0");
            hits.push(RawHit {
                line: n + 1,
                col: col(line, m.start()),
                length: m.as_str().chars().count(),
                values: captures(&target, &caps),
            });
        }
    }
    hits
}

struct Block {
    line: usize,
    col: usize,
    lines: usize,
}

/// `engine.comment-blocks`: a run of comment lines with one marker, or one
/// block comment, over its limit. The header is the first block after the preamble.
pub fn comment_blocks(
    src: &Source,
    lang: &Lang,
    params: &BTreeMap<String, ParamValue>,
) -> Vec<RawHit> {
    let markers = param_list(params, "markers");
    let preamble = lang::preamble_len(lang, &src.lines);
    let mut blocks: Vec<Block> = Vec::new();
    let mut last: Option<(usize, String, Option<usize>)> = None;
    for c in &src.comments {
        if !c.own_line || c.line < preamble || (!markers.is_empty() && !markers.contains(&c.marker))
        {
            continue;
        }
        let joins = match &last {
            Some((line, marker, block)) => match (block, c.block) {
                (Some(a), Some(b)) => *a == b,
                (None, None) => *line + 1 == c.line && *marker == c.marker,
                _ => false,
            },
            None => false,
        };
        if joins && let Some(b) = blocks.last_mut() {
            b.lines = c.line - b.line + 1;
        } else {
            blocks.push(Block {
                line: c.line,
                col: c.col,
                lines: 1,
            });
        }
        last = Some((c.line, c.marker.clone(), c.block));
    }
    let header = param_num(params, "header");
    let block = param_num(params, "block");
    let mut hits = Vec::new();
    for (i, b) in blocks.iter().enumerate() {
        let limit = if i == 0 && b.line == preamble {
            header
        } else {
            block
        };
        if b.lines as f64 > limit {
            hits.push(RawHit {
                line: b.line + 1,
                col: b.col + 1,
                length: 0,
                values: BTreeMap::from([
                    ("lines".to_string(), Value::Number(b.lines as f64)),
                    ("limit".to_string(), Value::Number(limit)),
                ]),
            });
        }
    }
    hits
}

/// One `necro-allow <code>[: reason]` in a comment.
#[derive(Clone, Debug)]
pub struct Allow {
    pub line: usize,
    pub col: usize,
    pub length: usize,
    pub code: String,
    pub reason: Option<String>,
}

const ALLOW: &str = "necro-allow";

pub fn allows(src: &Source) -> Vec<Allow> {
    let mut out = Vec::new();
    for c in src.comments.iter().filter(|c| !c.docstring) {
        let text = &c.text;
        let starts: Vec<usize> = text.match_indices(ALLOW).map(|(i, _)| i).collect();
        for (k, &start) in starts.iter().enumerate() {
            let end = starts.get(k + 1).copied().unwrap_or(text.len());
            let body = &text[start + ALLOW.len()..end];
            let trimmed = body.trim_start();
            let code: String = trimmed
                .chars()
                .take_while(|ch| {
                    ch.is_ascii_lowercase() || ch.is_ascii_digit() || *ch == '-' || *ch == '/'
                })
                .collect();
            if code.is_empty() || !body.starts_with(char::is_whitespace) {
                continue;
            }
            let after = &trimmed[code.len()..];
            let reason = after
                .strip_prefix(':')
                .map(|r| {
                    r.trim()
                        .trim_end_matches(';')
                        .trim_end_matches("*/")
                        .trim_end_matches("-->")
                        .trim()
                })
                .filter(|r| !r.is_empty())
                .map(String::from);
            let length = ALLOW.len() + (body.len() - trimmed.len()) + code.len();
            out.push(Allow {
                line: c.line + 1,
                col: c.text_col + text[..start].chars().count() + 1,
                length,
                code,
                reason,
            });
        }
    }
    out
}
