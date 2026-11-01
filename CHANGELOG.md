# Changelog

## Unreleased

- README lists `uv tool`, `pip` and `cargo` installs, and its pre-commit snippet no longer pins a version range.

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
