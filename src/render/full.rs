//! The `full` format (spec: Messages): progressive, so the first hit of a rule
//! teaches and the repeats stay short.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use crate::run::{Diagnostic, Report};

/// The column `why` and `fix` wrap at.
pub const WIDTH: usize = 88;
const REPEATS: usize = 4;
const TAB: &str = "    ";

/// Word-wrap `text` after `prefix`, continuation lines aligned under the text.
pub fn wrap(prefix: &str, text: &str) -> String {
    let indent = " ".repeat(prefix.chars().count());
    let mut out = String::new();
    for (i, para) in text.trim_end().split('\n').enumerate() {
        let mut line = if i == 0 {
            prefix.to_string()
        } else {
            indent.clone()
        };
        let mut empty = true;
        for word in para.split(' ').filter(|w| !w.is_empty()) {
            let fits = line.chars().count() + 1 + word.chars().count() <= WIDTH;
            if !empty && !fits {
                out.push_str(line.trim_end());
                out.push('\n');
                line = indent.clone();
                empty = true;
            }
            if !empty {
                line.push(' ');
            }
            line.push_str(word);
            empty = false;
        }
        out.push_str(line.trim_end());
        out.push('\n');
    }
    out
}

fn location(d: &Diagnostic) -> String {
    if d.line == 0 {
        d.file.clone()
    } else {
        format!("{}:{}:{}", d.file, d.line, d.col)
    }
}

/// The source line with tabs as 4 spaces, and the underline's offset and width to match.
fn snippet(source: &str, col: usize, length: usize) -> (String, usize, usize) {
    let width = |c: char| if c == '\t' { 4 } else { 1 };
    let chars: Vec<char> = source.chars().collect();
    let start: usize = chars
        .iter()
        .take(col.saturating_sub(1))
        .map(|&c| width(c))
        .sum();
    let span: usize = chars
        .iter()
        .skip(col.saturating_sub(1))
        .take(length)
        .map(|&c| width(c))
        .sum();
    (source.replace('\t', TAB), start, span.max(1))
}

/// The `full` format: the first hit teaches, repeats stay short.
pub fn render(report: &Report) -> String {
    let mut out = String::new();
    let mut shown: BTreeMap<&str, usize> = BTreeMap::new();
    let mut folded: Vec<(&Diagnostic, usize)> = Vec::new();
    let diags = &report.diagnostics;
    for (i, d) in diags.iter().enumerate() {
        let head = format!("error[{}]", d.rule);
        let n = shown.entry(&d.rule).or_default();
        *n += 1;
        if *n == 1 {
            first(&mut out, &head, d);
        } else if *n <= REPEATS + 1 {
            let label = if d.label.is_empty() {
                String::new()
            } else {
                format!(" ({})", d.label)
            };
            let _ = writeln!(out, "{head}: also at {}{label}", location(d));
            out.push_str(&wrap("   = fix: ", &d.fix));
            out.push('\n');
        } else {
            match folded.last_mut() {
                Some((last, n)) if last.rule == d.rule && last.file == d.file => *n += 1,
                _ => folded.push((d, 1)),
            }
        }
        // The folded counts for a rule close its run.
        let run_ends = diags.get(i + 1).is_none_or(|n| n.rule != d.rule);
        if run_ends && !folded.is_empty() {
            for (f, n) in folded.drain(..) {
                let _ = writeln!(out, "error[{}]: {n} more in {}", f.rule, f.file);
            }
            out.push('\n');
        }
    }
    out.push_str(&summary(report));
    out
}

fn first(out: &mut String, head: &str, d: &Diagnostic) {
    let _ = writeln!(out, "{head}: {}", d.message);
    let w = if d.line == 0 {
        1
    } else {
        d.line.to_string().len()
    };
    let pad = " ".repeat(w);
    match &d.source {
        None => {
            let label = if d.label.is_empty() {
                String::new()
            } else {
                format!(" ({})", d.label)
            };
            let _ = writeln!(out, "{pad}--> {}{label}", location(d));
        }
        Some(source) => {
            let _ = writeln!(out, "{pad}--> {}", location(d));
            let (line, start, span) = snippet(source, d.col, d.length);
            let underlined: String = source
                .chars()
                .skip(d.col.saturating_sub(1))
                .take(d.length)
                .collect();
            let label = if d.label.is_empty() || d.label == underlined {
                String::new()
            } else {
                format!(" {}", d.label)
            };
            let _ = writeln!(out, "{pad} |");
            let _ = writeln!(out, "{} | {}", d.line, line);
            let _ = writeln!(
                out,
                "{pad} | {}{}{label}",
                " ".repeat(start),
                "^".repeat(span)
            );
            let _ = writeln!(out, "{pad} |");
        }
    }
    out.push_str(&wrap(&format!("{pad} = why: "), &d.why));
    out.push_str(&wrap(&format!("{pad} = fix: "), &d.fix));
    let _ = writeln!(out, "{pad} = more: necro --explain {}", d.rule);
    out.push('\n');
}

fn counts<'a>(rules: impl Iterator<Item = &'a str>) -> String {
    let mut by: BTreeMap<&str, usize> = BTreeMap::new();
    for r in rules {
        *by.entry(r).or_default() += 1;
    }
    by.iter()
        .map(|(c, n)| format!("{c} ×{n}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn plural(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// The counts per rule and the allows behind a report.
pub fn summary(report: &Report) -> String {
    let mut out = String::new();
    let errors = &report.diagnostics;
    if !errors.is_empty() {
        let n = plural(errors.len(), "error", "errors");
        let by = counts(errors.iter().map(|d| d.rule.as_str()));
        let _ = writeln!(out, "necro: {n} ({by})");
    }
    for a in &report.allowed {
        let reason = if a.reason.is_empty() {
            "(no reason)"
        } else {
            &a.reason
        };
        let at = if a.line == 0 {
            a.file.clone()
        } else {
            format!("{}:{}:{}", a.file, a.line, a.col)
        };
        let _ = writeln!(out, "necro: allowed {} at {at}: {reason}", a.rule);
    }
    out
}
