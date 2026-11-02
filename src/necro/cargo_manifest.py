#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">= 3.9"
# dependencies = []
# ///
"""Workspace manifests for the cargo-manifest rule in rust.yaml.

Usage: cargo_manifest.py

Reads `cargo metadata` (so `edition.workspace = true` resolves) for every
workspace that workspaces.py finds, and reports each member under the working
folder (or holding it, as a root in src/ is held by ../Cargo.toml) whose edition is older than 2024, at its `edition` line, and each
publishable member with no `rust-version`, as a whole-file hit. Paths are
relative to the working folder. Exit 0; 2 when cargo fails.
"""

import json
import os
import re
import subprocess
import sys

sys.dont_write_bytecode = True  # no __pycache__ beside the rules
from workspaces import inside, workspaces

EDITION = "2024"


def members(root):
    """The workspace's member packages, from `cargo metadata`."""
    run = subprocess.run(
        ["cargo", "metadata", "--no-deps", "--format-version", "1"],
        capture_output=True, text=True, cwd=root,
    )
    if run.returncode:
        print(run.stderr.strip(), file=sys.stderr)
        sys.exit(2)
    meta = json.loads(run.stdout)
    ids = set(meta["workspace_members"])
    return [pkg for pkg in meta["packages"] if pkg["id"] in ids]


def main():
    hits = []
    for pkg in (pkg for root in workspaces() for pkg in members(root)):
        manifest = pkg["manifest_path"]
        path = os.path.relpath(manifest)
        holds_root = inside(os.path.relpath(".", os.path.dirname(manifest)))
        if not (inside(path) or holds_root):
            continue
        if pkg["edition"] < EDITION:
            with open(manifest, encoding="utf-8") as f:
                lines = f.read().splitlines()
            at = next((i for i, l in enumerate(lines, 1) if re.match(r"\s*edition\b", l)), None)
            where = f"{path}:{at}:1" if at else path
            hits.append(f"{where}: {pkg['name']} is on edition {pkg['edition']}")
        if pkg["rust_version"] is None and pkg["publish"] != []:
            hits.append(f"{path}: {pkg['name']} is publishable with no rust-version")
    for hit in sorted(hits):
        print(hit)


if __name__ == "__main__":
    main()
