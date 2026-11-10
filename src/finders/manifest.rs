//! `necro cargo-manifest` (spec: Finders): crate manifests below the house
//! baseline, the current edition and a declared `rust-version`.

use crate::args::{self, Spec};
use crate::command::Hit;
use std::path::Path;

use super::cargo::{relative, workspaces};

const USAGE: &str = "necro cargo-manifest [--edition <year>]";

/// The edition a member must be on unless `--edition` says otherwise. It
/// moves with necro releases, not with the installed toolchain, so every
/// machine gives the same answer.
const EDITION: &str = "2024";

/// The `cargo-manifest` finder: every member manifest under the working folder.
pub fn run(args: &[String]) -> Result<Vec<Hit>, String> {
    let parsed = args::parse(
        args,
        &Spec {
            values: &["--edition"],
            flags: &[],
            pathspecs: false,
        },
    )
    .map_err(|e| e.with_usage(USAGE))?;
    if let Some(extra) = parsed.positional.first() {
        return Err(format!("unexpected argument {extra}: {USAGE}"));
    }
    let want = parsed.value("--edition").unwrap_or(EDITION);
    if want.len() != 4 || !want.chars().all(|c| c.is_ascii_digit()) {
        return Err(format!("--edition takes a year such as 2024, got {want}"));
    }
    let mut hits: Vec<Hit> = Vec::new();
    for ws in workspaces()? {
        for pkg in &ws.members {
            let manifest = Path::new(pkg["manifest_path"].as_str().unwrap_or_default());
            let Some(path) = relative(manifest) else {
                continue;
            };
            let name = pkg["name"].as_str().unwrap_or_default();
            let edition = pkg["edition"].as_str().unwrap_or("2015");
            if edition < want {
                let text = format!("{name} is on edition {edition}");
                match edition_line(manifest) {
                    Some(n) => hits.push(Hit {
                        path: path.clone(),
                        line: n,
                        col: 1,
                        endcol: None,
                        label: text,
                    }),
                    None => hits.push(Hit {
                        path: path.clone(),
                        line: 0,
                        col: 0,
                        endcol: None,
                        label: text,
                    }),
                }
            }
            // `publish = false` reads as an empty registry list.
            let publishable = pkg["publish"].as_array().is_none_or(|r| !r.is_empty());
            if pkg["rust_version"].is_null() && publishable {
                let text = format!("{name} is publishable with no rust-version");
                hits.push(Hit {
                    path,
                    line: 0,
                    col: 0,
                    endcol: None,
                    label: text,
                });
            }
        }
    }
    hits.sort_by(|a, b| a.path.cmp(&b.path).then(a.line.cmp(&b.line)));
    Ok(hits)
}

/// The 1-based line of the manifest's `edition` key, if it has one.
fn edition_line(manifest: &Path) -> Option<usize> {
    let text = std::fs::read_to_string(manifest).ok()?;
    text.lines()
        .position(|l| {
            l.trim_start()
                .strip_prefix("edition")
                .is_some_and(|rest| !rest.starts_with(|c: char| c.is_alphanumeric() || c == '_'))
        })
        .map(|i| i + 1)
}
