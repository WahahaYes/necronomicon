---
name: necro-dev
description: Change necronomicon itself, its engine, CLI, finders, packs or spec. Use when adding or changing a necro.yaml key, a finder (necro <name>), a built-in pack rule, output wording, or a conformance case in this repo.
metadata:
  internal: true
---

# Developing necronomicon

The order is always spec, then cases, then engine. The spec ([spec/v0.md](../../../spec/v0.md)) says what necro does; the cases in `tests/conformance/cases/` prove it; the engine follows.

## Writing a case

A case is one behavior, `cases/<area>/<name>.case`, in the format [tests/conformance/README.md](../../../tests/conformance/README.md) describes: a `#` line saying what it shows, `spec:` naming the section, `args:`, `exit:`, then files and the expected `@stdout`, `@stderr` or `@config-error`.

- Hits sort by rule name, then path, line and column; `rust-style/…` sorts before `rust/…`, since `-` comes before `/`.
- `--format terse` keeps expectations short; use the full format only when the layout is what the case shows.
- `@config-error` lines are `file:line:col: key.path`, the position of the key or value at fault.
- Run one area while working: `uv run --script tests/conformance/run.py -k <area> -- target/debug/necro` (pass `--engine-version <x.y.z>` when a case prints the version). `make test` runs everything.

## Adding a rule key

The key goes in `RULE_KEYS` (`src/config.rs`), `load_rule` parses and validates it, and the spec's rule table gains a row. Add cases for the key working, for a bad value (a config error at the right position), and for `--explain` if it shows the key.

## Adding a finder

- `src/finders/<name>.rs` with a `run(args) -> Result<Vec<String>, String>` that returns hit lines, `path[:line[:col[-endcol]]][: label]`, sorted; register it in `FINDERS` (`src/finders/mod.rs`).
- A finder that reads files belongs in `FILE_FINDERS` (`src/run.rs`), so a narrowed run passes it `--files-from`; take that option and keep only the listed files.
- A finder that can fix belongs in `FIX_FINDERS` (`src/config.rs`) with the option that marks a fixable command; `--fix` adds `--write` and `--only-at <file>`.
- Errors are usage errors (`Err`), never hits. A tool it runs that's missing says how to install it.
- A spec section under Finders gives its usage line and each check's hit format.

## Adding a pack rule

- Packs are `src/packs/<pack>.yaml` (only `rules:`), registered in `src/packs/mod.rs`; `Cargo.toml`'s `include` must cover the file.
- Commands use only finders. Give every rule `files:` so a narrowed run skips it, and make limits `params:` a repo can override.
- Opinions go in an opt-in companion pack (`<lang>-style`), not in the core one.
- Pack text can't spell the allow syntax, or the tracked YAML parses as an allow: say "an allow comment".
- Check a new rule against a real adopter before shipping it: count its hits and read a few.

## Changing a shipped skill

`necro`, `necro-rules` and `necro-adopt` are read in other repos: links are absolute GitHub URLs, and the allow syntax is written with placeholders (`<rule>`), since these files are under this repo's root. `necro` and `necro-rules` are embedded by `src/skills.rs` and listed in `Cargo.toml`'s `include`. Don't run `necro skills install` here; it would stamp the sources.

## Before committing

`make format`, `make test`, `make lint`; `make check` runs every hook as CI does. One concern per commit; a change that touches the spec, cases and engine for one behavior is one concern.
