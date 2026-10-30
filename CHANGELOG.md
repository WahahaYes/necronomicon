# Changelog

## Unreleased

## 0.1.0 - 2026-10-30

- Rules are commands that print `path[:line[:col[-endcol]]][:label]`, with `message`, `why`, `fix` and optional `details` and `examples`; `include:` splits them across files.
- `necro grep`, the first bundled finder: Rust regex, code-point columns, `--skip-comments`.
- Allows (`necro-allow <rule>: <reason>`).
- `full` and `terse` output, `--explain`, `--rule`; the `necro` binary packaged as a wheel.
- Wheels on PyPI for Linux (x86_64, aarch64), macOS (arm64, x86_64) and Windows (x64), and the crate on crates.io.
