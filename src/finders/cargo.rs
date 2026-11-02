//! What the Cargo finders share (spec: Finders): every workspace under the
//! working folder, read through `cargo metadata`, and paths relative to it.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::Value;

/// One Cargo workspace, from `cargo metadata --no-deps`.
pub struct Workspace {
    pub root: PathBuf,
    pub target: PathBuf,
    /// The member packages' metadata objects.
    pub members: Vec<Value>,
}

/// The workspace enclosing the working folder, if any, plus the workspace of
/// each tracked `Cargo.toml` below it, shallowest first. A manifest that
/// belongs to a workspace already found costs no extra cargo call.
pub fn workspaces() -> Result<Vec<Workspace>, String> {
    let listed = crate::files::git()
        .args(["ls-files", "-z"])
        .output()
        .map_err(|e| format!("can't run git: {e}"))?;
    let mut manifests: Vec<PathBuf> = String::from_utf8_lossy(&listed.stdout)
        .split('\0')
        .filter(|p| Path::new(p).file_name().is_some_and(|n| n == "Cargo.toml"))
        .map(PathBuf::from)
        .collect();
    manifests.sort_by_key(|p| p.components().count());
    let mut found: Vec<Workspace> = Vec::new();
    let mut covered: BTreeSet<PathBuf> = BTreeSet::new();
    let folders = std::iter::once(PathBuf::from(".")).chain(
        manifests
            .iter()
            .map(|m| m.parent().map_or(PathBuf::from("."), Path::to_path_buf)),
    );
    for folder in folders {
        let Ok(folder) = std::fs::canonicalize(&folder) else {
            continue;
        };
        if covered.contains(&folder.join("Cargo.toml")) {
            continue;
        }
        let Some(root) = locate(&folder) else {
            continue;
        };
        if found.iter().any(|w| w.root == root) {
            continue;
        }
        let ws = metadata(&root)?;
        for m in &ws.members {
            if let Some(path) = m["manifest_path"].as_str() {
                covered.insert(PathBuf::from(path));
            }
        }
        found.push(ws);
    }
    Ok(found)
}

/// The root folder of the workspace `folder` belongs to, or `None` outside any.
fn locate(folder: &Path) -> Option<PathBuf> {
    let out = Command::new("cargo")
        .args(["locate-project", "--workspace", "--message-format", "plain"])
        .current_dir(folder)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let manifest = PathBuf::from(String::from_utf8_lossy(&out.stdout).trim());
    manifest.parent().map(Path::to_path_buf)
}

fn metadata(root: &Path) -> Result<Workspace, String> {
    let out = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(root)
        .output()
        .map_err(|e| format!("can't run cargo: {e}"))?;
    if !out.status.success() {
        return Err(String::from_utf8_lossy(&out.stderr).trim().to_string());
    }
    let meta: Value = serde_json::from_slice(&out.stdout)
        .map_err(|e| format!("can't read cargo metadata: {e}"))?;
    let ids: BTreeSet<&str> = meta["workspace_members"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    let members = meta["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|p| p["id"].as_str().is_some_and(|id| ids.contains(id)))
        .cloned()
        .collect();
    let path = |key: &str| PathBuf::from(meta[key].as_str().unwrap_or_default());
    Ok(Workspace {
        root: canonical(&path("workspace_root")),
        target: path("target_directory"),
        members,
    })
}

fn canonical(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// `path` relative to the working folder, with `/` separators, or `None`
/// when it isn't under it.
pub fn relative(path: &Path) -> Option<String> {
    let here = std::env::current_dir().ok()?;
    let rel = canonical(path)
        .strip_prefix(canonical(&here))
        .ok()?
        .to_path_buf();
    let parts: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    (!parts.is_empty()).then(|| parts.join("/"))
}

/// The folder under a workspace's target folder that `necro clippy` builds in.
pub fn cache(ws: &Workspace) -> PathBuf {
    ws.target.join("necro")
}

/// Remove every workspace's cache folder: each as (folder, size), the folder
/// relative to the working folder when it's under it.
pub fn clean_cache() -> Result<Vec<(String, String)>, String> {
    let mut removed = Vec::new();
    for ws in workspaces()? {
        let folder = cache(&ws);
        if !folder.is_dir() {
            continue;
        }
        let size = size(&folder);
        std::fs::remove_dir_all(&folder)
            .map_err(|e| format!("can't remove {}: {e}", folder.display()))?;
        let shown = relative(&folder).unwrap_or_else(|| folder.display().to_string());
        removed.push((shown, human(size)));
    }
    Ok(removed)
}

/// The bytes in the files under `path`, not following links.
fn size(path: &Path) -> u64 {
    let Ok(meta) = std::fs::symlink_metadata(path) else {
        return 0;
    };
    if !meta.is_dir() {
        return meta.len();
    }
    std::fs::read_dir(path)
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| size(&entry.path()))
        .sum()
}

/// A size in bytes, as `7 B` or `1.9 GB` (1024-based).
fn human(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["KB", "MB", "GB", "TB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64 / 1024.0;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.1} {}", UNITS[unit])
}
