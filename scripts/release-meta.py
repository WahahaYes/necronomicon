#!/usr/bin/env python3
"""Release metadata shared by scripts/release.sh and the release workflow.

  release-meta.py check X.Y.Z   exit 1 unless Cargo.toml, Cargo.lock and the
                                stamped plugin manifests carry X.Y.Z (the
                                wheel takes Cargo.toml's version)
  release-meta.py stamp X.Y.Z   write X.Y.Z into the plugin manifests' version
                                and re-copy .agents/skills into every harness
                                plugin skills/ dir; same commit as the Cargo
                                bump in release.sh open_pr
  release-meta.py sync-skills [--check] [FILE…]
                                re-copy .agents/skills into every harness
                                plugin skills/ dir (--check prints each
                                drifted copy instead, for the
                                plugin-skills-sync rule; extra files, from
                                an autofix {files} expansion, are ignored:
                                the re-copy is total)
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
                                docs, where a bump would leave it behind.
                                A stamp already at the current Cargo.toml
                                version is the release's own, not a hit
"""
import os
import re
import json
import subprocess
import sys
from pathlib import Path

# RELEASE_ROOT points it at another checkout, such as release.sh's scratch
# worktree.
ROOT = Path(os.environ.get("RELEASE_ROOT") or Path(__file__).resolve().parent.parent)
CHANGELOG = ROOT / "CHANGELOG.md"
# Where a hardcoded version would reach users. The manifests, the lockfile
# and the CHANGELOG carry versions on purpose.
STALE_PATHS = ["src", "README.md", "docs", "spec", "examples", ".pre-commit-hooks.yaml",
               "plugins", ".agents/plugins"]
# Harness plugin manifests stamped with Cargo.toml's version at release
# (release.sh stamps them, check verifies them). Marketplace entries stay
# unversioned: the manifest wins and a mismatch would only warn.
STAMP_FILES = [
    "plugins/necronomicon/.claude-plugin/plugin.json",
    ".agents/plugins/necronomicon/plugin.json",
    "plugins/opencode-necronomicon/package.json",
]
# Skills carried for coding agents, re-copied from the canonical
# .agents/skills into every harness plugin skills/ dir at release.
STAMP_SKILLS = ["necro", "necro-rules", "necro-adopt"]
STAMP_SKILL_DIRS = [
    "plugins/necronomicon/skills",
    ".agents/plugins/necronomicon/skills",
    "plugins/opencode-necronomicon/skills",
]


def check(ver):
    bad = []
    # [package] comes first, so its version is the file's first.
    cargo = re.search(r'^version = "([^"]*)"$', (ROOT / "Cargo.toml").read_text(), re.M)
    if not cargo or cargo.group(1) != ver:
        bad.append("Cargo.toml")
    lock = re.search(r'^name = "necronomicon"\nversion = "([^"]*)"$', (ROOT / "Cargo.lock").read_text(), re.M)
    if not lock or lock.group(1) != ver:
        bad.append("Cargo.lock")
    for rel in STAMP_FILES:
        try:
            stamped = json.loads((ROOT / rel).read_text()).get("version")
        except (OSError, ValueError):
            stamped = None
        if stamped != ver:
            bad.append(rel)
    for f in bad:
        print("release-meta: %s is not at %s" % (f, ver), file=sys.stderr)
    return 1 if bad else 0


def derived(name):
    """The plugin copy of a canonical skill: identical but for a header
    comment pointing at the file to edit, kept after the frontmatter so
    harnesses still parse it."""
    lines = (ROOT / ".agents/skills" / name / "SKILL.md").read_text().split("\n")
    if not lines or lines[0].strip() != "---" or "---" not in lines[1:]:
        raise ValueError(".agents/skills/%s/SKILL.md has no frontmatter" % name)
    end = lines.index("---", 1)
    rest = lines[end + 1:]
    if rest and rest[0] == "":
        rest = rest[1:]
    header = "<!-- Derived from .agents/skills/%s/SKILL.md: edit the canonical file there, then re-copy here. -->" % name
    return "\n".join(lines[: end + 1] + ["", header, ""] + rest)


def sync_skills(*args):
    """Re-copy .agents/skills into every harness plugin skills/ dir. With
    --check, print each drifted copy, `path: label`, instead of writing
    (the plugin-skills-sync rule's command). Any other arguments are an
    autofix {files} expansion and are ignored: the re-copy is total and
    idempotent."""
    if args[:1] == ("--check",):
        return check_skills()
    for folder in STAMP_SKILL_DIRS:
        for name in STAMP_SKILLS:
            try:
                text = derived(name)
            except ValueError as e:
                print("release-meta: %s" % e, file=sys.stderr)
                return 1
            (ROOT / folder / name).mkdir(parents=True, exist_ok=True)
            (ROOT / folder / name / "SKILL.md").write_text(text)
    return 0


def check_skills():
    """Print each plugin skill copy that differs from its canonical skill.
    A missing copy is reported on its canonical skill, since a hit's path
    must name a file that exists."""
    for name in STAMP_SKILLS:
        try:
            want = derived(name)
        except ValueError as e:
            print("release-meta: %s" % e, file=sys.stderr)
            return 2
        canonical = (ROOT / ".agents/skills" / name / "SKILL.md").relative_to(ROOT).as_posix()
        for folder in STAMP_SKILL_DIRS:
            copy = ROOT / folder / name / "SKILL.md"
            rel = copy.relative_to(ROOT).as_posix()
            if not copy.exists():
                print("%s: plugin copy %s is missing" % (canonical, rel))
            elif copy.read_text() != want:
                print("%s: differs from %s" % (rel, canonical))
    return 0


def stamp(ver):
    for rel in STAMP_FILES:
        target = ROOT / rel
        try:
            data = json.loads(target.read_text())
        except (OSError, ValueError) as e:
            print("release-meta: can't stamp %s: %s" % (rel, e), file=sys.stderr)
            return 1
        data["version"] = ver
        target.write_text(json.dumps(data, indent=2) + "\n")
    return sync_skills()


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
    cargo = re.search(r'^version = "([^"]*)"$', (ROOT / "Cargo.toml").read_text(), re.M)
    current = cargo.group(1) if cargo else None
    paths = [p for p in STALE_PATHS if (ROOT / p).exists()]
    pattern = []
    for v in versions:
        pattern += ["-e", v]
    out = subprocess.run(
        ["git", "-C", str(ROOT), "grep", "-n", "-w", "-F"] + pattern + ["--"] + paths,
        capture_output=True, text=True,
    ).stdout
    failed = False
    for line in out.splitlines():
        path, _, rest = line.partition(":")
        _, _, text = rest.partition(":")
        # A stamp already at the current version is the release's own: the
        # harness reads it from the manifest, never from the build. A stamp
        # at any other released version is still a hit.
        if (path in STAMP_FILES and current
                and re.fullmatch(r'\s*"version": "%s",?' % re.escape(current), text)):
            continue
        print("release-meta: version hardcoded here: %s" % line, file=sys.stderr)
        failed = True
    return 1 if failed else 0


if __name__ == "__main__":
    cmds = {"check": (check, 1), "stamp": (stamp, 1), "sync-skills": (sync_skills, 0, float("inf")),
            "notes": (notes, 1), "changelog": (changelog, 2, 3), "stale": (stale, 0)}
    args = sys.argv[2:]
    spec = cmds.get(sys.argv[1] if len(sys.argv) > 1 else "")
    if not spec or not (spec[1] <= len(args) <= spec[-1]):
        sys.exit(__doc__.strip())
    sys.exit(spec[0](*args))
