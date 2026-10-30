# /// script
# requires-python = ">=3.11"
# ///
"""Run the conformance cases against an engine: run.py [-k TEXT] [-v] [--engine-version V] -- <engine argv>.

Each case lays its files over base/ in a fresh temp dir and runs the engine
there. Format and comparison rules: README.md. Stdlib only.
"""

import os
import re
import shlex
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

HERE = Path(__file__).resolve().parent
SECTION = re.compile(r"^-- (.+?) --$")
HEADER_KEYS = {"spec", "args", "exit", "env", "git", "needs", "stdin"}


def parse(path):
    """Split a case into its header, files and expected sections."""
    header, files, expect = {}, {}, {}
    current, body = None, []

    def flush():
        if current is None:
            return
        text = "\n".join(body)
        if text and not text.endswith("\n"):
            text += "\n"
        (expect if current.startswith("@") else files)[current] = text

    for line in path.read_text(encoding="utf-8").split("\n"):
        m = SECTION.match(line)
        if m:
            flush()
            current, body = m.group(1), []
        elif current is not None:
            body.append(line)
        elif line.startswith("#") or not line.strip():
            continue
        else:
            key, _, value = line.partition(":")
            if key not in HEADER_KEYS:
                raise ValueError(f"{path}: unknown header key {key!r}")
            header[key] = value.strip()
    flush()
    # A section body ends at the next marker; drop the blank line before it.
    files = {k: v[:-1] if v.endswith("\n\n") else v for k, v in files.items()}
    return header, files, expect


def normalise(text, root, version=None):
    text = text.replace("\r\n", "\n").replace(str(root), "<root>")
    if version:
        text = text.replace(version, "<version>")
    lines = [line.rstrip() for line in text.split("\n")]
    while lines and not lines[-1]:
        lines.pop()
    return "\n".join(lines)


def build_tree(root, files, git):
    shutil.copytree(HERE / "base", root, dirs_exist_ok=True)
    for rel, text in files.items():
        target = root / rel
        if not text:
            if target.exists():
                target.unlink()
            continue
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(text, encoding="utf-8")
    if git:
        quiet = {"cwd": root, "check": True, "stdout": subprocess.DEVNULL}
        subprocess.run(["git", "init", "-q"], **quiet)
        subprocess.run(["git", "add", "-A"], **quiet)


def check_config_errors(expected, stderr):
    """@config-error lists `file:line:col: key.path`, one per stderr line."""
    want = [w for w in expected.strip().split("\n") if w]
    got = [g for g in stderr.split("\n") if g]
    if len(want) != len(got):
        return f"want {len(want)} config error line(s), got {len(got)}"
    for w, g in zip(want, got):
        if not g.startswith(f"necro: config error: {w}: "):
            return f"want 'necro: config error: {w}: …'\n     got {g!r}"
    return None


def run_case(path, engine, bin_dir, version):
    header, files, expect = parse(path)
    with tempfile.TemporaryDirectory(prefix="necro-case-") as tmp:
        root = Path(tmp).resolve() / "repo"
        build_tree(root, files, header.get("git") != "false")
        env = {k: v for k, v in os.environ.items() if k not in ("CI", "NO_COLOR")}
        env["NO_COLOR"] = "1"
        env["PATH"] = f"{bin_dir}{os.pathsep}{env.get('PATH', '')}"
        for pair in shlex.split(header.get("env", "")):
            key, _, value = pair.partition("=")
            env[key] = value.replace("<root>", str(root))
        args = shlex.split(header.get("args", ""))
        try:
            proc = subprocess.run(
                engine + args, cwd=root, env=env, capture_output=True, text=True, timeout=20,
                input=header.get("stdin", ""),
            )
        except subprocess.TimeoutExpired:
            return ["timed out after 20 s"]
        out, err = normalise(proc.stdout, root, version), normalise(proc.stderr, root, version)
        problems = []
        if "@config-error" in expect:
            want_exit = 2
            if out:
                problems.append(f"stdout should be empty:\n{out}")
            problem = check_config_errors(expect["@config-error"], err)
            if problem:
                problems.append(f"{problem}\nstderr:\n{err}")
        else:
            want_exit = int(header.get("exit", "0"))
            for name, got in (("@stdout", out), ("@stderr", err)):
                want = normalise(expect.get(name, ""), root)
                if got != want:
                    problems.append(diff(name, want, got))
        if proc.returncode != want_exit:
            problems.append(f"exit {proc.returncode}, want {want_exit}" + (f"\nstderr:\n{err}" if err else ""))
        return problems


def diff(name, want, got):
    import difflib

    lines = difflib.unified_diff(want.split("\n"), got.split("\n"), "want", "got", lineterm="")
    return f"{name} differs:\n" + "\n".join(lines)


def main(argv):
    if "--" not in argv:
        sys.exit(__doc__.split("\n")[0])
    split = argv.index("--")
    opts, engine = argv[:split], argv[split + 1 :]
    pattern = opts[opts.index("-k") + 1] if "-k" in opts else ""
    verbose = "-v" in opts
    version = opts[opts.index("--engine-version") + 1] if "--engine-version" in opts else None
    # The engine is on PATH as `necro`, as an installed hook would be.
    bin_dir = Path(tempfile.mkdtemp(prefix="necro-bin-"))
    exe = Path(shutil.which(engine[0]) or engine[0]).resolve()
    engine = [str(exe)] + engine[1:]
    (bin_dir / "necro").symlink_to(exe)
    cases = sorted(p for p in (HERE / "cases").rglob("*.case") if pattern in str(p))
    failed = 0
    for case in cases:
        name = case.relative_to(HERE / "cases").with_suffix("").as_posix()
        problems = run_case(case, engine, bin_dir, version)
        if problems:
            failed += 1
            print(f"FAIL {name}")
            for problem in problems:
                print("  " + problem.replace("\n", "\n  "))
        elif verbose:
            print(f"ok   {name}")
    shutil.rmtree(bin_dir)
    print(f"conformance: {len(cases) - failed} of {len(cases)} passed")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
