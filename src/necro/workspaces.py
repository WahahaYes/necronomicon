"""Cargo workspaces for the Rust rules in rust.yaml (imported by clippy.py and
cargo_manifest.py).

A rule runs in necro's root, which may sit inside a workspace (necronomicon's
src/) or above several (a repo whose Rust lives in src-tauri/, with a nested
workspace that pins its own toolchain). workspaces() finds them all, so the
same rule text works in either layout.
"""

import os
import subprocess


def locate(folder):
    """The workspace root folder for `folder`, or None outside any workspace."""
    run = subprocess.run(
        ["cargo", "locate-project", "--workspace", "--message-format", "plain"],
        capture_output=True, text=True, cwd=folder,
    )
    return os.path.dirname(run.stdout.strip()) if run.returncode == 0 else None


def workspaces():
    """Workspace root folders, sorted: the one enclosing the working folder,
    plus the workspace of every tracked Cargo.toml below it. Run cargo with
    each as its working folder, so rustup picks up that workspace's pin."""
    found = {locate(".")}
    listed = subprocess.run(
        ["git", "ls-files", "-z", "--", "Cargo.toml", "**/Cargo.toml"],
        capture_output=True, text=True,
    )
    for manifest in filter(None, listed.stdout.split("\0")):
        found.add(locate(os.path.dirname(manifest) or "."))
    found.discard(None)
    return sorted(found)


def inside(path):
    """Whether `path` (relative to the working folder) is under it."""
    return path != ".." and not path.startswith(".." + os.sep)
