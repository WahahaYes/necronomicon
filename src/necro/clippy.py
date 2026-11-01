#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">= 3.9"
# dependencies = []
# ///
"""Clippy lints as necro hits, for the Rust rules in rust.yaml.

Usage: clippy.py [--tests] [--match RE] [--skip RE] LINT...

Runs `cargo clippy --message-format=json` with every lint in PACK at warn, so
all rules share one cached run (two with --tests, which adds test targets),
in every workspace that workspaces.py finds, and prints the requested lints'
primary spans under the working folder as `path:line:col-endcol: msg`, paths
relative to it. --match / --skip keep or drop hits by the span's source text.
Exit 0; 2 on a usage or build error.
"""

import json
import os
import re
import subprocess
import sys

sys.dont_write_bytecode = True  # no __pycache__ beside the rules
from workspaces import inside, workspaces

PACK = [
    "unwrap_used",
    "expect_used",
    "allow_attributes",
    "allow_attributes_without_reason",
    "todo",
    "unimplemented",
    "dbg_macro",
    "undocumented_unsafe_blocks",
    "let_underscore_must_use",
    "unused_result_ok",
    "cast_possible_truncation",
]


def usage(what):
    print(f"clippy.py: {what}", file=sys.stderr)
    sys.exit(2)


def parse(argv):
    tests, keep, drop, lints = False, None, None, []
    args = iter(argv)
    for a in args:
        if a == "--tests":
            tests = True
        elif a in ("--match", "--skip"):
            pattern = next(args, None) or usage(f"{a} needs a regex")
            if a == "--match":
                keep = re.compile(pattern)
            else:
                drop = re.compile(pattern)
        elif a in PACK:
            lints.append(a)
        else:
            usage(f"not a pack lint: {a}")
    return tests, keep, drop, lints or usage("no lints given")


def user_span(message, root):
    """The primary span in the workspace, following macro expansions out."""
    for span in message["spans"]:
        while span and span["is_primary"]:
            if not os.path.isabs(span["file_name"]):
                return span
            span = (span.get("expansion") or {}).get("span")
    return None


def clippy(root, tests, keep, drop, wanted, hits):
    """Add one workspace's hits to `hits`; return its errors as text."""
    cmd = ["cargo", "clippy", "--quiet", "--message-format=json"]
    if tests:
        cmd.append("--all-targets")
    cmd += ["--"] + [f"-Wclippy::{lint}" for lint in PACK]
    run = subprocess.run(cmd, capture_output=True, text=True, cwd=root)
    errors = []
    for line in run.stdout.splitlines():
        msg = json.loads(line)
        if msg.get("reason") != "compiler-message":
            continue
        diag = msg["message"]
        if diag["level"] == "error":
            errors.append(diag.get("rendered") or diag["message"])
        if (diag.get("code") or {}).get("code") not in wanted:
            continue
        span = user_span(diag, root)
        if span is None:
            continue
        path = os.path.relpath(os.path.join(root, span["file_name"]))
        if not inside(path):
            continue
        # `text` holds whole source lines; the span is the highlighted part.
        text = "\n".join(
            t["text"][t["highlight_start"] - 1 : t["highlight_end"] - 1] for t in span["text"]
        ).strip()
        if (keep and not keep.search(text)) or (drop and drop.search(text)):
            continue
        col = f"{span['column_start']}"
        if span["line_start"] == span["line_end"] and span["column_end"] > span["column_start"] + 1:
            col += f"-{span['column_end'] - 1}"
        # Two of a rule's lints on one line (allow_attributes and its
        # _without_reason twin) are one violation, at the leftmost column.
        key = (path, span["line_start"])
        if key not in hits or span["column_start"] < hits[key][0]:
            hits[key] = (span["column_start"], col, diag["message"])
    if run.returncode or errors:
        return "".join(errors) or run.stderr[-2000:]
    return ""


def main():
    tests, keep, drop, lints = parse(sys.argv[1:])
    roots = workspaces() or usage("no Cargo workspace here")
    wanted = {f"clippy::{lint}" for lint in lints}
    hits, failed = {}, []
    for root in roots:
        error = clippy(root, tests, keep, drop, wanted, hits)
        if error:
            failed.append(f"{os.path.relpath(root)}: {error}")
    if failed:
        print("\n".join(failed), file=sys.stderr)
        sys.exit(2)
    for (path, line), (_, col, text) in sorted(hits.items()):
        print(f"{path}:{line}:{col}: {text}")


if __name__ == "__main__":
    main()
