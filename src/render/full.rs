//! The `full` format (spec: Messages): progressive, so the first hit of a code
//! teaches and the repeats stay short.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use crate::run::{Diagnostic, Report};

pub const WIDTH: usize = 88;
const REPEATS: usize = 4;
const TAB: &str = "    ";

/// Whether `necro` is on PATH, so the explain line can be run as written.
pub fn necro_on_path() -> bool {
    let name = if cfg!(windows) { "necro.exe" } else { "necro" };
    std::env::var_os("PATH")
        .is_some_and(|p| std::env::split_paths(&p).any(|d| d.join(name).is_file()))
}

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

pub fn render(report: &Report, on_path: bool) -> String {
    let mut out = String::new();
    let mut shown: BTreeMap<&str, (usize, BTreeSet<&str>, BTreeSet<&str>)> = BTreeMap::new();
    let mut folded: Vec<(&Diagnostic, usize)> = Vec::new();
    let diags = &report.diagnostics;
    for (i, d) in diags.iter().enumerate() {
        let head = format!("{}[{}]", d.severity.name(), d.code);
        let entry = shown.entry(&d.code).or_default();
        entry.0 += 1;
        if entry.0 == 1 {
            entry.1.insert(&d.summary);
            entry.2.insert(&d.why);
            first(&mut out, &head, d, on_path);
        } else if entry.0 <= REPEATS + 1 {
            if entry.1.insert(&d.summary) {
                let _ = writeln!(out, "{head}: {}\n  --> {}", d.summary, location(d));
            } else {
                let _ = writeln!(out, "{head}: also at {}", location(d));
            }
            if entry.2.insert(&d.why) {
                out.push_str(&wrap("   = why: ", &d.why));
            }
            out.push_str(&wrap("   = help: ", &d.help));
            out.push('\n');
        } else {
            match folded.last_mut() {
                Some((last, n))
                    if last.code == d.code
                        && last.file == d.file
                        && last.severity == d.severity =>
                {
                    *n += 1
                }
                _ => folded.push((d, 1)),
            }
        }
        // The folded counts for a code close its run.
        let run_ends = diags
            .get(i + 1)
            .is_none_or(|n| n.code != d.code || n.severity != d.severity);
        if run_ends && !folded.is_empty() {
            for (f, n) in folded.drain(..) {
                let _ = writeln!(
                    out,
                    "{}[{}]: {n} more in {}",
                    f.severity.name(),
                    f.code,
                    f.file
                );
            }
            out.push('\n');
        }
    }
    out.push_str(&summary(report));
    out
}

fn first(out: &mut String, head: &str, d: &Diagnostic, on_path: bool) {
    let _ = writeln!(out, "{head}: {}", d.summary);
    let w = if d.line == 0 {
        1
    } else {
        d.line.to_string().len()
    };
    let pad = " ".repeat(w);
    let _ = writeln!(out, "{pad}--> {}", location(d));
    if let Some(source) = &d.source {
        let (line, start, span) = snippet(source, d.col, d.length);
        let label = d
            .label
            .as_ref()
            .map(|l| format!(" {l}"))
            .unwrap_or_default();
        let _ = writeln!(out, "{pad} |");
        let _ = writeln!(out, "{} | {}", d.line, line).map(|_| ());
        let _ = writeln!(
            out,
            "{pad} | {}{}{label}",
            " ".repeat(start),
            "^".repeat(span)
        );
        let _ = writeln!(out, "{pad} |");
    }
    out.push_str(&wrap(&format!("{pad} = why: "), &d.why));
    out.push_str(&wrap(&format!("{pad} = help: "), &d.help));
    let more = format!("necro --explain {}", d.code);
    let _ = writeln!(out, "{pad} = more: {more}");
    if !on_path {
        let install = format!(
            "(necro isn't on PATH: uvx --from necronomicon=={} {more})",
            env!("CARGO_PKG_VERSION")
        );
        out.push_str(&wrap(&format!("{pad}         "), &install));
    }
    out.push('\n');
}

fn counts(diags: &[&Diagnostic]) -> String {
    let mut by: BTreeMap<&str, usize> = BTreeMap::new();
    for d in diags {
        *by.entry(&d.code).or_default() += 1;
    }
    by.iter()
        .map(|(c, n)| format!("{c} ×{n}"))
        .collect::<Vec<_>>()
        .join(", ")
}

pub fn summary(report: &Report) -> String {
    let mut out = String::new();
    let (errors, experimental): (Vec<&Diagnostic>, Vec<&Diagnostic>) = report
        .diagnostics
        .iter()
        .partition(|d| d.severity == crate::config::Severity::Error);
    if !errors.is_empty() {
        let s = if errors.len() == 1 { "" } else { "s" };
        let _ = writeln!(
            out,
            "necro: {} error{s} ({})",
            errors.len(),
            counts(&errors)
        );
    }
    if !experimental.is_empty() {
        let _ = writeln!(
            out,
            "necro: {} experimental, not failing ({})",
            experimental.len(),
            counts(&experimental)
        );
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
            format!("{}:{}", a.file, a.line)
        };
        let _ = writeln!(out, "necro: allowed {} at {at}: {reason}", a.code);
    }
    for w in &report.warnings {
        let _ = writeln!(out, "necro: warning: {w}");
    }
    out
}
