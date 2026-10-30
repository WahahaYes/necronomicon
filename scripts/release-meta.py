#!/usr/bin/env python3
"""Release metadata shared by scripts/release.sh and the release workflow.

  release-meta.py check X.Y.Z   exit 1 unless Cargo.toml and Cargo.lock carry
                                X.Y.Z (the wheel takes Cargo.toml's version)
  release-meta.py notes X.Y.Z   print the CHANGELOG.md section for X.Y.Z;
                                exit 1 when there is none
  release-meta.py changelog X.Y.Z DATE [NOTES_FILE]
                                give CHANGELOG.md a "## X.Y.Z - DATE" section:
                                NOTES_FILE's text if given (replacing an
                                existing X.Y.Z or Unreleased section), else
                                keep an existing X.Y.Z section, else rename
                                "## Unreleased"; exit 1 when none of these
                                yields notes. Leaves an empty "## Unreleased"
                                on top for the next release's notes
  release-meta.py stale         exit 1, listing each hit, if any version in
                                CHANGELOG.md is written into source or user
                                docs, where a bump would leave it behind
"""
import os
import re
import subprocess
import sys
from pathlib import Path

# RELEASE_ROOT points it at another checkout, such as release.sh's scratch
# worktree.
ROOT = Path(os.environ.get("RELEASE_ROOT") or Path(__file__).resolve().parent.parent)
CHANGELOG = ROOT / "CHANGELOG.md"
# Where a hardcoded version would reach users. The manifests, the lockfile
# and the CHANGELOG carry versions on purpose.
STALE_PATHS = ["src", "README.md", "docs", "spec", "examples", ".pre-commit-hooks.yaml"]


def check(ver):
    bad = []
    # [package] comes first, so its version is the file's first.
    cargo = re.search(r'^version = "([^"]*)"$', (ROOT / "Cargo.toml").read_text(), re.M)
    if not cargo or cargo.group(1) != ver:
        bad.append("Cargo.toml")
    lock = re.search(r'^name = "necronomicon"\nversion = "([^"]*)"$', (ROOT / "Cargo.lock").read_text(), re.M)
    if not lock or lock.group(1) != ver:
        bad.append("Cargo.lock")
    for f in bad:
        print("release-meta: %s is not at %s" % (f, ver), file=sys.stderr)
    return 1 if bad else 0


def section(text, heading):
    """(start, end, body) of the "## <heading>" section, or None."""
    m = re.search(r"^## %s(?: [^\n]*)?\n(.*?)(?=^## |\Z)" % re.escape(heading), text, re.S | re.M)
    return (m.start(), m.end(), m.group(1).strip()) if m else None


def notes(ver):
    found = section(CHANGELOG.read_text(), ver)
    if not found or not found[2]:
        print("release-meta: no CHANGELOG section for %s" % ver, file=sys.stderr)
        return 1
    print(found[2])
    return 0


def changelog(ver, date, notes_file=None):
    text = CHANGELOG.read_text()
    head = "## %s - %s" % (ver, date)
    mine, unreleased = section(text, ver), section(text, "Unreleased")
    if notes_file:
        body = Path(notes_file).read_text().strip()
        if not body:
            print("release-meta: %s is empty" % notes_file, file=sys.stderr)
            return 1
    elif mine and mine[2]:
        print("release-meta: keeping the existing CHANGELOG section for %s" % ver, file=sys.stderr)
        return 0
    elif unreleased and unreleased[2]:
        body = unreleased[2]
    else:
        print("release-meta: no notes for %s: write them under ## Unreleased or pass a notes file" % ver,
              file=sys.stderr)
        return 1
    new = "%s\n\n%s\n\n" % (head, body)
    # The released section takes the place of this version's old section,
    # else of Unreleased, else goes above the newest release.
    target = mine or unreleased
    if target:
        text = text[:target[0]] + new + text[target[1]:]
    else:
        first = re.search(r"^## ", text, re.M)
        at = first.start() if first else len(text)
        text = text[:at] + new + text[at:]
    # Keep an Unreleased heading on top, so PRs add their notes under it
    # instead of each adding the heading (which conflicts between PRs).
    if not section(text, "Unreleased"):
        first = re.search(r"^## ", text, re.M)
        at = first.start() if first else len(text)
        text = text[:at] + "## Unreleased\n\n" + text[at:]
    # One newline at the end, even when the new section is the last.
    CHANGELOG.write_text(text.rstrip("\n") + "\n")
    return 0


def stale():
    versions = re.findall(r"^## (\d+\.\d+\.\d+)\b", CHANGELOG.read_text(), re.M)
    if not versions:
        return 0
    paths = [p for p in STALE_PATHS if (ROOT / p).exists()]
    pattern = []
    for v in versions:
        pattern += ["-e", v]
    out = subprocess.run(
        ["git", "-C", str(ROOT), "grep", "-n", "-w", "-F"] + pattern + ["--"] + paths,
        capture_output=True, text=True,
    ).stdout
    hits = out.splitlines()
    for line in hits:
        print("release-meta: version hardcoded here: %s" % line, file=sys.stderr)
    return 1 if hits else 0


if __name__ == "__main__":
    cmds = {"check": (check, 1), "notes": (notes, 1), "changelog": (changelog, 2, 3), "stale": (stale, 0)}
    args = sys.argv[2:]
    spec = cmds.get(sys.argv[1] if len(sys.argv) > 1 else "")
    if not spec or not (spec[1] <= len(args) <= spec[-1]):
        sys.exit(__doc__.strip())
    sys.exit(spec[0](*args))
