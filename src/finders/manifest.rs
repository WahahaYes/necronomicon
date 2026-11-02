//! `necro cargo-manifest` (spec: Finders): crate manifests below the house
//! baseline, the current edition and a declared `rust-version`.

use std::path::Path;

use super::cargo::{relative, workspaces};

const USAGE: &str = "necro cargo-manifest";

/// The edition a member must be on. It moves with necro releases, not with
/// the installed toolchain, so every machine gives the same answer.
const EDITION: &str = "2024";

/// The `cargo-manifest` finder: every member manifest under the working folder.
pub fn run(args: &[String]) -> Result<Vec<String>, String> {
    if let Some(arg) = args.first() {
        return Err(format!("unexpected argument {arg}: {USAGE}"));
    }
    // (path, line, text): whole-file hits are line 0, so they sort first.
    let mut hits: Vec<(String, usize, String)> = Vec::new();
    for ws in workspaces()? {
        for pkg in &ws.members {
            let manifest = Path::new(pkg["manifest_path"].as_str().unwrap_or_default());
            let Some(path) = relative(manifest) else {
                continue;
            };
            let name = pkg["name"].as_str().unwrap_or_default();
            let edition = pkg["edition"].as_str().unwrap_or("2015");
            if edition < EDITION {
                let text = format!("{name} is on edition {edition}");
                match edition_line(manifest) {
                    Some(n) => hits.push((path.clone(), n, format!("{path}:{n}:1: {text}"))),
                    None => hits.push((path.clone(), 0, format!("{path}: {text}"))),
                }
            }
            // `publish = false` reads as an empty registry list.
            let publishable = pkg["publish"].as_array().is_none_or(|r| !r.is_empty());
            if pkg["rust_version"].is_null() && publishable {
                let text = format!("{path}: {name} is publishable with no rust-version");
                hits.push((path, 0, text));
            }
        }
    }
    hits.sort();
    Ok(hits.into_iter().map(|(_, _, text)| text).collect())
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
