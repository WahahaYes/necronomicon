//! `necro ty` (spec: Finders): ty's type-checking diagnostics over tracked
//! `.py` and `.pyi` files, as hits. ty comes from `PATH`;
//! `necronomicon[python]` installs the pinned version beside necro.

use std::collections::HashSet;
use std::process::Command;

use serde_json::Value;

use super::grep::read_list;
use super::ruff::{at, diagnostics, here, py_files, relative};
use crate::args::{self, Spec};
use crate::command::Hit;

const USAGE: &str = "necro ty [--files-from <file>] [--except=<rule>,…] [-- <pathspec>…]";

/// Files per ty run, so a large repo stays under the argument limit.
const CHUNK: usize = 400;

/// The `ty` finder.
pub fn run(args: &[String]) -> Result<Vec<Hit>, String> {
    let parsed = args::parse(
        args,
        &Spec {
            values: &["--files-from", "--except"],
            flags: &[],
            pathspecs: true,
        },
    )
    .map_err(|e| e.with_usage(USAGE))?;
    if let Some(extra) = parsed.positional.first() {
        return Err(format!("unexpected argument {extra}: {USAGE}"));
    }
    let except: HashSet<String> = parsed
        .values("--except")
        .into_iter()
        .flat_map(|list| list.split(',').map(|r| r.trim().to_string()))
        .filter(|r| !r.is_empty())
        .collect();
    let only = parsed.value("--files-from").map(read_list).transpose()?;
    let files = py_files(&parsed.pathspecs, only.as_ref())?;
    if files.is_empty() {
        return Ok(vec![]);
    }
    let here = here()?;
    let mut hits = Vec::new();
    for chunk in files.chunks(CHUNK) {
        let output = match Command::new("ty")
            .args(["check", "--force-exclude", "--output-format", "gitlab"])
            .args(chunk)
            .output()
        {
            Ok(o) => o,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(
                    "ty isn't on PATH; install necronomicon[python], which pins ty beside necro"
                        .to_string(),
                );
            }
            Err(e) => return Err(format!("can't run ty: {e}")),
        };
        let code = output.status.code().unwrap_or(-1);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        for d in diagnostics("ty", code, &stdout, &stderr, &[0, 1])? {
            let rule = d
                .get("check_name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if except.contains(rule) {
                continue;
            }
            let location = d.get("location").cloned().unwrap_or(Value::Null);
            let Some(file) = location.get("path").and_then(Value::as_str) else {
                continue;
            };
            let positions = location.get("positions").cloned().unwrap_or(Value::Null);
            let begin = positions.get("begin").cloned().unwrap_or(Value::Null);
            let end = positions.get("end").cloned().unwrap_or(Value::Null);
            let (line, col) = (at(&begin, "line"), at(&begin, "column"));
            // ty's end column is exclusive; necro's endcol is inclusive.
            let last = at(&end, "column").saturating_sub(1);
            let endcol = (at(&end, "line") == line && last > col).then_some(last);
            let description = d
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or_default();
            // gitlab's description repeats the rule as a prefix.
            let message = description
                .strip_prefix(&format!("{rule}: "))
                .unwrap_or(description);
            hits.push(Hit {
                path: relative(&here, file),
                line,
                col,
                endcol,
                label: format!("{message} ({rule})"),
            });
        }
    }
    Ok(hits)
}
