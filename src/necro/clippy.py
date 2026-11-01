#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">= 3.9"
# dependencies = []
# ///
"""Clippy lints as necro hits, for the Rust rules in rust.yaml.

Usage: clippy.py [--tests] [--match RE] [--skip RE] LINT...

Runs `cargo clippy --message-format=json` with every lint in PACK at warn, so
all rules share one cached run (two with --tests, which adds test targets),
and prints the requested lints' primary spans as `path:line:col-endcol: msg`,
paths relative to the working folder. --match / --skip keep or drop hits by
the span's source text. Exit 0; 2 on a usage or build error.
"""

import json
import os
import re
import subprocess
import sys

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


def main():
    tests, keep, drop, lints = parse(sys.argv[1:])
    root = subprocess.run(
        ["cargo", "locate-project", "--workspace", "--message-format", "plain"],
        capture_output=True, text=True,
    )
    if root.returncode:
        usage(root.stderr.strip() or "no Cargo workspace here")
    root = os.path.dirname(root.stdout.strip())
    cmd = ["cargo", "clippy", "--quiet", "--message-format=json"]
    if tests:
        cmd.append("--all-targets")
    cmd += ["--"] + [f"-Wclippy::{lint}" for lint in PACK]
    run = subprocess.run(cmd, capture_output=True, text=True)
    wanted = {f"clippy::{lint}" for lint in lints}
    hits, errors = {}, []
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
        # `text` holds whole source lines; the span is the highlighted part.
        text = "\n".join(
            t["text"][t["highlight_start"] - 1 : t["highlight_end"] - 1] for t in span["text"]
        ).strip()
        if (keep and not keep.search(text)) or (drop and drop.search(text)):
            continue
        path = os.path.relpath(os.path.join(root, span["file_name"]))
        col = f"{span['column_start']}"
        if span["line_start"] == span["line_end"] and span["column_end"] > span["column_start"] + 1:
            col += f"-{span['column_end'] - 1}"
        # Two of a rule's lints on one line (allow_attributes and its
        # _without_reason twin) are one violation, at the leftmost column.
        key = (path, span["line_start"])
        if key not in hits or span["column_start"] < hits[key][0]:
            hits[key] = (span["column_start"], col, diag["message"])
    if run.returncode or errors:
        print("".join(errors) or run.stderr[-2000:], file=sys.stderr)
        sys.exit(2)
    for (path, line), (_, col, text) in sorted(hits.items()):
        print(f"{path}:{line}:{col}: {text}")


if __name__ == "__main__":
    main()
