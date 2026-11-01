#!/usr/bin/env -S uv run --script
# /// script
# requires-python = ">= 3.9"
# dependencies = []
# ///
"""Docstrings for necro's own house rules (docstring-missing, docstring-too-long).

Usage: docstrings.py (missing|too-long)

Scans every *.rs file under the working folder (necro runs commands with the
rule root as cwd) in one pass. `missing` reports each public item with no doc
comment; `too-long` reports each doc block over 50 words. Hits print as
`path:line:col: label`; exit 0 with hits, 1 clean, 2 on a usage error.
"""

import re
import sys
from pathlib import Path

LIMIT = 50

ITEM_RE = re.compile(
    r"^\s*pub\s+"
    r"(?:(?:unsafe|async|const)\s+)*"
    r"(?P<kind>fn|struct|enum|trait|type|const|static|mod)\b"
    r"\s*(?P<name>[A-Za-z_][A-Za-z0-9_]*)?"
)
MOD_RE = re.compile(r"^\s*(?:pub\s+)?mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*(;|\{)")
WORDS_RE = re.compile(r"\S+")


def code_part(line):
    """The line without its // comment; ' is a lifetime unless it closes soon."""
    out = []
    i = 0
    in_str = None
    while i < len(line):
        c = line[i]
        if in_str:
            out.append(c)
            if c == "\\":
                if i + 1 < len(line):
                    out.append(line[i + 1])
                    i += 1
            elif c == in_str:
                in_str = None
        elif c in ("\"", "'"):
            if c == "'" and not (
                (i + 2 < len(line) + 1 and line[i + 2 : i + 3] == "'")
                or (line[i + 1 : i + 2] == "\\" and line[i + 3 : i + 4] == "'")
            ):
                out.append(c)  # a lifetime like 'a: skip the quote, keep text
            else:
                in_str = c
                out.append(c)
        elif c == "/" and line[i + 1 : i + 2] == "/":
            break
        else:
            out.append(c)
        i += 1
    return "".join(out)


def depth_delta(code):
    """Brace depth change on one comment-stripped line."""
    depth = 0
    for c in code:
        if c in "{[(":
            depth += 1
        elif c in "}])":
            depth -= 1
    return depth


def block_words(doc_lines):
    """Words in doc lines, markers stripped, fenced code excluded."""
    text = []
    for line in doc_lines:
        s = line.strip()
        if s.startswith("///"):
            s = s[3:]
        elif s.startswith("//!"):
            s = s[3:]
        else:
            continue
        if s.startswith(" "):
            s = s[1:]
        text.append(s)
    words = 0
    in_fence = False
    for line in text:
        if line.strip().startswith("```"):
            in_fence = not in_fence
            continue
        if not in_fence:
            words += len(WORDS_RE.findall(line))
    return words


def scan(path):
    """One pass: items with their doc words, every doc block's words."""
    lines = path.read_text(encoding="utf-8").splitlines()
    attr_line = [False] * len(lines)
    in_attr = 0
    for i, line in enumerate(lines):
        s = line.strip()
        if in_attr > 0:
            attr_line[i] = True
            in_attr += depth_delta(code_part(line))
            if in_attr <= 0:
                in_attr = 0
        elif s.startswith("#["):
            attr_line[i] = True
            in_attr = depth_delta(code_part(line))
            if in_attr <= 0:
                in_attr = 0

    blocks = []  # (start_index, words) for every /// or //! run
    i = 0
    while i < len(lines):
        s = lines[i].strip()
        if s.startswith("///") or s.startswith("//!"):
            j = i
            while j + 1 < len(lines) and (
                lines[j + 1].strip().startswith("///")
                if s.startswith("///")
                else lines[j + 1].strip().startswith("//!")
            ):
                j += 1
            blocks.append((i, block_words(lines[i : j + 1])))
            i = j + 1
        else:
            i += 1

    items = []  # (line_no, col, kind, name, words|None, guarded)
    mod_stack = []  # (depth_after_open, is_test)
    depth = 0
    pending_test = False
    for i, line in enumerate(lines):
        s = line.strip()
        m = ITEM_RE.match(line)
        if m and not s.startswith("//"):
            j = i - 1
            words = None
            guarded = False
            while j >= 0 and (lines[j].strip() == "" or attr_line[j]):
                if attr_line[j]:
                    t = lines[j].strip()
                    if "cfg(test)" in t or t == "#[test]":
                        guarded = True
                j -= 1
            if j >= 0 and lines[j].strip().startswith("///"):
                k = j
                while k - 1 >= 0 and lines[k - 1].strip().startswith("///"):
                    k -= 1
                words = block_words(lines[k : j + 1])
            if any(test for _, test in mod_stack):
                guarded = True
            col = sum(1 for _ in line[: line.index("pub")]) + 1
            items.append((i + 1, col, m.group("kind"), m.group("name"), words, guarded))

        if s.startswith("#["):
            if "cfg(test)" in s:
                pending_test = True
        elif s == "":
            pass
        else:
            mm = MOD_RE.match(code_part(line))
            if mm and mm.group(2) == "{":
                mod_stack.append((depth + depth_delta(code_part(line)), pending_test))
            pending_test = False

        depth += depth_delta(code_part(line))
        while mod_stack and depth < mod_stack[-1][0]:
            mod_stack.pop()

    return items, blocks


def module_has_inner_doc(path, mod_name):
    """True when the module's file starts with an inner doc comment."""
    sibling = path.parent / (mod_name + ".rs")
    nested = path.parent / mod_name / "mod.rs"
    for cand in (sibling, nested):
        if cand.is_file():
            try:
                first = cand.read_text(encoding="utf-8").splitlines()
            except OSError:
                continue
            if first and first[0].strip().startswith("//!"):
                return True
    return False


def main(argv):
    """Run one mode over the tree; hits exit 0, clean exits 1."""
    if len(argv) != 2 or argv[1] not in ("missing", "too-long"):
        sys.stderr.write("usage: docstrings.py (missing|too-long)\n")
        return 2
    mode = argv[1]
    root = Path.cwd()
    try:
        files = sorted(root.rglob("*.rs"))
    except OSError as e:
        sys.stderr.write("docstrings.py: can't list files: %s\n" % e)
        return 2
    hits = []
    for path in files:
        rel = path.relative_to(root).as_posix()
        try:
            items, blocks = scan(path)
        except (OSError, UnicodeError) as e:
            sys.stderr.write("docstrings.py: can't read %s: %s\n" % (rel, e))
            return 2
        if mode == "missing":
            for line_no, col, kind, name, words, guarded in items:
                if guarded or words:
                    continue
                if kind == "mod" and name and module_has_inner_doc(path, name):
                    continue
                label = "%s %s" % (kind, name) if name else kind
                hits.append("%s:%d:%d: %s" % (rel, line_no, col, label))
        else:
            seen = set()
            for line_no, col, kind, name, words, _guarded in items:
                if words is not None and words > LIMIT and line_no not in seen:
                    seen.add(line_no)
                    label = "%s %s" % (kind, name) if name else kind
                    hits.append(
                        "%s:%d:%d: %s doc is %d words (limit %d)"
                        % (rel, line_no, col, label, words, LIMIT)
                    )
            for start, words in blocks:
                if words > LIMIT and ("block", start) not in seen:
                    seen.add(("block", start))
                    hits.append(
                        "%s:%d:1: doc block is %d words (limit %d)"
                        % (rel, start + 1, words, LIMIT)
                    )
    for hit in hits:
        print(hit)
    return 0 if hits else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv))
