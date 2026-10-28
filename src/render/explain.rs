//! `--explain <code|group>` (spec: CLI): the long form an agent reads after a hit.

use std::fmt::Write as _;

use crate::config::{Config, ParamValue, Rule};
use crate::run::render_param;

fn param_list(params: &std::collections::BTreeMap<String, ParamValue>) -> String {
    let show = |v: &ParamValue| match v {
        ParamValue::StrMap(m) => format!(
            "{{{}}}",
            m.iter()
                .map(|(k, v)| format!("{k}: {v}"))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        ParamValue::StrList(l) => format!("[{}]", l.join(", ")),
        v => render_param(v),
    };
    params
        .iter()
        .map(|(k, v)| format!("{k}: {}", show(v)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn rule(r: &Rule) -> String {
    let mut out = String::new();
    let summary = r.wording.summary.as_deref().unwrap_or("");
    let _ = writeln!(out, "{} ({}): {summary}\n", r.code, r.severity.name());
    let _ = writeln!(out, "{}\n", r.explain.trim_end());
    if !r.params.is_empty() {
        let _ = writeln!(out, "params: {}", param_list(&r.params));
    }
    for w in r.wheres.iter().filter(|w| !w.params.is_empty()) {
        let _ = writeln!(
            out,
            "  in {}: {}",
            w.files_src.join(", "),
            param_list(&w.params)
        );
    }
    let _ = writeln!(out, "group: {} (necro --explain {})", r.group, r.group);
    out
}

/// The text for `name`, or the known codes and groups when there's no such thing.
pub fn render(config: &Config, name: &str) -> Result<String, String> {
    if let Some(r) = config.rules.iter().find(|r| r.code == name) {
        return Ok(rule(r));
    }
    if let Some(g) = config.groups.iter().find(|g| g.name == name) {
        let rules: Vec<&Rule> = config.rules.iter().filter(|r| r.group == g.name).collect();
        let mut out = String::new();
        let s = if rules.len() == 1 { "" } else { "s" };
        let _ = writeln!(out, "{}: {} rule{s}\n", g.name, rules.len());
        let _ = writeln!(out, "{}\n", g.guide.trim_end());
        let width = rules.iter().map(|r| r.code.len()).max().unwrap_or(0);
        for r in rules {
            let _ = writeln!(
                out,
                "  {:width$}  {}",
                r.code,
                r.wording.summary.as_deref().unwrap_or("")
            );
        }
        return Ok(out);
    }
    let codes: Vec<&str> = config.rules.iter().map(|r| r.code.as_str()).collect();
    let groups: Vec<&str> = config.groups.iter().map(|g| g.name.as_str()).collect();
    Err(format!(
        "no rule or group `{name}`; rules: {}; groups: {}",
        codes.join(", "),
        groups.join(", ")
    ))
}
