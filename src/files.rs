//! Finding the root and the files a run looks at (spec: Files; Scope and parameters).

use std::path::{Path, PathBuf};
use std::process::Command;

/// The nearest folder at or above `start` that holds `necro.yaml`.
pub fn find_root(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|d| d.join("necro.yaml").is_file())
        .map(Path::to_path_buf)
}

/// Every file under `root`, root-relative with forward slashes: from git when
/// `root` is inside a work tree, else a walk that respects `.gitignore`.
pub fn list(root: &Path) -> Vec<String> {
    let git = Command::new("git")
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .current_dir(root)
        .output();
    let mut files: Vec<String> = match git {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .split('\0')
            .filter(|p| !p.is_empty() && root.join(p).is_file())
            .map(String::from)
            .collect(),
        _ => walk(root),
    };
    files.sort();
    files.dedup();
    files
}

fn walk(root: &Path) -> Vec<String> {
    let walker = ignore::WalkBuilder::new(root)
        .hidden(false)
        .require_git(false)
        .filter_entry(|e| e.file_name() != ".git")
        .build();
    walker
        .flatten()
        .filter(|e| e.file_type().is_some_and(|t| t.is_file()))
        .filter_map(|e| rel(root, e.path()))
        .collect()
}

/// `path` relative to `root`, with forward slashes, if it's inside.
pub fn rel(root: &Path, path: &Path) -> Option<String> {
    let r = path.strip_prefix(root).ok()?;
    let parts: Vec<String> = r
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    (!parts.is_empty()).then(|| parts.join("/"))
}

/// Make `path` absolute against `cwd` and drop `.` and `..` without touching the disk.
pub fn absolute(cwd: &Path, path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in cwd.join(path).components() {
        match c {
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::CurDir => {}
            c => out.push(c),
        }
    }
    out
}
