//! `necro rustfmt` (spec: Finders): the files `cargo fmt --check` would
//! change in each workspace, and `--write` to reformat them.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::cargo::{Workspace, relative, workspaces};
use super::grep::read_list;

const USAGE: &str = "necro rustfmt [--write [--only-at <file>]]";

/// The `rustfmt` finder.
pub fn run(args: &[String]) -> Result<Vec<String>, String> {
    let mut write = false;
    let mut only_at: Option<String> = None;
    let mut it = args.iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--write" => write = true,
            "--only-at" => {
                only_at = Some(
                    it.next()
                        .ok_or(format!("--only-at needs a value: {USAGE}"))?
                        .clone(),
                );
            }
            a if a.starts_with("--only-at=") => only_at = Some(a["--only-at=".len()..].to_string()),
            // A narrowed run's list: rustfmt checks whole workspaces.
            "--files-from" => {
                it.next();
            }
            other => return Err(format!("unknown option {other}: {USAGE}")),
        }
    }
    let only: Option<HashSet<String>> = only_at.as_deref().map(read_list).transpose()?.map(|l| {
        l.into_iter()
            .map(|h| h.split(':').next().unwrap_or_default().to_string())
            .collect()
    });
    let mut hits = Vec::new();
    for ws in workspaces()? {
        let files = unformatted(&ws)?;
        if !write {
            hits.extend(
                files
                    .iter()
                    .map(|(rel, _)| format!("{rel}: needs reformatting")),
            );
            continue;
        }
        let chosen: Vec<&(String, PathBuf)> = files
            .iter()
            .filter(|(rel, _)| only.as_ref().is_none_or(|o| o.contains(rel)))
            .collect();
        for (rel, abs) in chosen {
            reformat(&ws, abs)?;
            hits.push(format!("{rel}: reformatted"));
        }
    }
    hits.sort();
    Ok(hits)
}

/// The files under the working folder that `cargo fmt --check` would change:
/// (path relative to the working folder, absolute path).
fn unformatted(ws: &Workspace) -> Result<Vec<(String, PathBuf)>, String> {
    let out = cargo_fmt(&ws.root)
        .args(["fmt", "--all", "--check", "--message-format", "short"])
        .output()
        .map_err(|e| format!("can't run cargo fmt: {e}"))?;
    let stdout = String::from_utf8_lossy(&out.stdout);
    let files: Vec<PathBuf> = stdout
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(PathBuf::from)
        .collect();
    if !out.status.success() && files.is_empty() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(format!(
            "cargo fmt failed in {}: {}",
            ws.root.display(),
            stderr.trim()
        ));
    }
    Ok(files
        .into_iter()
        .filter_map(|abs| relative(&abs).map(|rel| (rel, abs)))
        .collect())
}

/// Run rustfmt on one file with its package's edition, from the workspace's
/// folder, so its toolchain pin and rustfmt.toml apply.
fn reformat(ws: &Workspace, file: &Path) -> Result<(), String> {
    let edition = ws
        .members
        .iter()
        .filter_map(|pkg| {
            let dir = Path::new(pkg["manifest_path"].as_str()?).parent()?;
            file.starts_with(dir)
                .then(|| (dir.components().count(), pkg["edition"].as_str()))
        })
        .max_by_key(|(depth, _)| *depth)
        .and_then(|(_, e)| e)
        .unwrap_or("2015")
        .to_string();
    let out = Command::new("rustfmt")
        .current_dir(&ws.root)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .args(["--edition", &edition])
        .arg(file)
        .output()
        .map_err(|e| format!("can't run rustfmt: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "rustfmt failed on {}: {}",
            file.display(),
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    Ok(())
}

/// `cargo` run in `dir`, so a toolchain pin there applies.
fn cargo_fmt(dir: &Path) -> Command {
    let mut cmd = Command::new("cargo");
    cmd.current_dir(dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE");
    cmd
}
