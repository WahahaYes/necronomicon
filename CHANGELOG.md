# Changelog

## Unreleased

- README lists `uv tool`, `pip` and `cargo` installs, and its pre-commit snippet no longer pins a version range.
- Packs: `include: [necro:rust]` adds rules built into necro, named `<pack>/<rule>`, and `disable: [<pack>/<rule>]` turns one off. The first is `necro:rust`: no-unwrap, no-expect, lint-override-reason, no-stubs, unsafe-safety-comment, swallowed-error, weasel-type-names, lossy-cast and cargo-manifest, mostly clippy lints with necro's wording. necronomicon's own Rust rules now come from it.
- Two Cargo finders: `necro clippy <lint>…` prints clippy lints as hits for every Cargo workspace under the root, building in its own `target/necro` so it never invalidates the repo's own clippy cache, and `necro cargo-manifest` reports crates below edition 2024 or publishable without a `rust-version`.
- A failed read of a rule command's output is a check error naming the program; it used to pass as empty output, so the rule looked clean.
- The crate declares `rust-version = "1.98"`, the pinned toolchain.
- Allows are found in commits made from a git worktree: necro's own `git` calls ignore `GIT_DIR` and `GIT_WORK_TREE`, as rule commands already did, so a hook no longer misses every allow when the root is below the repo's top.
- Commands no longer time out after 120 s (a cold `cargo` build could hit it): necro waits for every command and, past 60 s, names the rules still running on stderr. Commands now share necro's process group, so Ctrl-C stops them too; the `libc` dependency is gone.

## 0.2.0 - 2026-10-31

- Baseline ratchet for rolling out rules over existing violations: committed `necro.baseline.yaml` snapshots match post-allow hits by rule, path and fingerprint, so old hits stay silent while new ones fail; stale entries warn on stderr without failing.
- `necro [--rule <name>] --write-baseline` regenerates the snapshot (full regen replaces, `--rule` merges that rule's entries only); built-in `necro/*` diagnostics are never snapshotted or matched, and a corrupt snapshot is exit 2 on check runs.
- `docstring-missing` and `docstring-too-long` house rules over `src/necro/docstrings.py` (fifty-word budget), with the tree swept and documented so a hit is a regression; the script documents itself.
- README gains the baseline ratchet bullet pointing at the spec's Baseline section.

## 0.1.0 - 2026-10-30

- Rules are commands that print `path[:line[:col[-endcol]]][:label]`, with `message`, `why`, `fix` and optional `details` and `examples`; `include:` splits them across files.
- `necro grep`, the first bundled finder: Rust regex, code-point columns, `--skip-comments`.
- Allows (`necro-allow <rule>: <reason>`).
- `full` and `terse` output, `--explain`, `--rule`; the `necro` binary packaged as a wheel.
- Wheels on PyPI for Linux (x86_64, aarch64), macOS (arm64, x86_64) and Windows (x64), and the crate on crates.io.
