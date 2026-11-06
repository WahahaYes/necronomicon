//! Finding the root and the files under it (spec: Commands, where it runs).

use std::path::{Path, PathBuf};
use std::process::Command;

/// The nearest folder at or above `start` that holds `necro.yaml`.
pub fn find_root(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .find(|d| d.join("necro.yaml").is_file())
        .map(Path::to_path_buf)
}

/// `git` that finds the repo from its working folder, ignoring GIT_DIR and
/// GIT_WORK_TREE, which git sets for hooks in a worktree.
pub fn git() -> Command {
    let mut git = Command::new("git");
    git.env_remove("GIT_DIR").env_remove("GIT_WORK_TREE");
    git
}

/// Whether `root` is inside a git work tree.
pub fn in_git(root: &Path) -> bool {
    git()
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(root)
        .output()
        .is_ok_and(|o| o.status.success() && o.stdout.starts_with(b"true"))
}

/// Every file git knows under `root` (tracked or not ignored), root-relative.
pub fn list(root: &Path) -> Vec<String> {
    let out = git()
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .current_dir(root)
        .output();
    let mut files: Vec<String> = match out {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .split('\0')
            .filter(|p| !p.is_empty() && root.join(p).is_file())
            .map(String::from)
            .collect(),
        _ => vec![],
    };
    files.sort();
    files.dedup();
    files
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

/// A given file's path relative to `root` (with `../` when it's outside),
/// or `None` when it isn't a file: narrowed runs drop deleted files.
pub fn anchor(cwd: &Path, root: &Path, given: &Path) -> Option<String> {
    let path = absolute(cwd, given);
    if !path.is_file() {
        return None;
    }
    let root = absolute(cwd, root);
    let common = root
        .components()
        .zip(path.components())
        .take_while(|(a, b)| a == b)
        .count();
    let ups = root.components().count() - common;
    let rest: Vec<String> = path
        .components()
        .skip(common)
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    let mut parts = vec!["..".to_string(); ups];
    parts.extend(rest);
    Some(parts.join("/"))
}

/// Create the symlink `link` pointing at `target`.
#[cfg(unix)]
pub fn symlink(target: &str, link: &Path) -> Result<(), String> {
    std::os::unix::fs::symlink(target, link)
        .map_err(|e| format!("can't link {}: {e}", link.display()))
}

/// Create the symlink `link` pointing at `target`.
#[cfg(not(unix))]
pub fn symlink(_: &str, link: &Path) -> Result<(), String> {
    Err(format!(
        "can't create the link {} on this system; create it by hand",
        link.display()
    ))
}
