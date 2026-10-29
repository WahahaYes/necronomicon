# Changelog

## Unreleased

- Rules are commands that print `path[:line[:col[-endcol]]][:label]`, with `message`, `why`, `fix` and optional `details` and `examples`; `include:` splits them across files.
- `necro grep`, the first bundled finder: Rust regex, code-point columns, `--skip-comments`.
- Allows (`necro-allow <rule>: <reason>`), `necro baseline` and `necro.baseline`.
- `full` and `terse` output, `--explain`, `--rule`; the `necro` binary packaged as a wheel.
