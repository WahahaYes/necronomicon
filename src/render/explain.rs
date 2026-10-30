//! `--explain <rule>` (spec: CLI): the long form an agent reads after a hit.

use std::fmt::Write as _;

use crate::config::Rule;

/// Snippets indented 4 spaces, blank lines between them.
fn snippets(out: &mut String, title: &str, list: &[String]) {
    let _ = writeln!(out, "{title}:");
    for (i, s) in list.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        for line in s.trim_end().lines() {
            let _ = writeln!(out, "{}", format!("    {line}").trim_end());
        }
    }
    out.push('\n');
}

pub fn render(r: &Rule) -> String {
    let mut out = String::new();
    let _ = writeln!(out, "{}: {}\n", r.name, r.message.trim_end());
    let _ = writeln!(out, "Why: {}\n", r.why.trim_end());
    let _ = writeln!(out, "Fix: {}\n", r.fix.trim_end());
    if let Some(d) = &r.details {
        let _ = writeln!(out, "{}\n", d.trim_end());
    }
    if let Some(ex) = &r.examples {
        snippets(&mut out, "Bad", &ex.bad);
        snippets(&mut out, "Good", &ex.good);
    }
    match &r.command {
        Some(c) => {
            let _ = writeln!(out, "Command: {c}");
        }
        None => {
            let _ = writeln!(out, "Built into necro.");
        }
    }
    out
}
