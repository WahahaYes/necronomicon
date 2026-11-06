# Changelog

## Unreleased

- **`necro agent-files`** checks the files coding agents read: AGENTS.md and CLAUDE.md pairs, skills in `.agents/skills` and their `.claude/skills` links, skill frontmatter, other harnesses' instruction files, pointers to missing rules, and necro skills from another version; `--write` creates a missing CLAUDE.md or link.
- A rule's hit may name a symlink, as a whole-file hit.

- `necro --fix` counts every file a fix changed or created, not only the files with hits.

- **Fixes.** `necro --fix` fixes the hits a run reports, then checks again and says what it changed and which hits it couldn't fix; it exits 1 when it changed a file, so a hook fails until the fixed files are re-staged. Allowed and baselined hits are left alone. The published hook and the README's now run `necro --fix`.
- **`necro grep --replace <text>`** makes a rule fixable: its labels say what each match becomes (`$1` for groups), and `--write` (which `--fix` adds) replaces them in place. Any other command names its fix in the new `autofix:` key, with `{files}` for the files to fix.
- A full run says how many of its hits `--fix` could clear, and `--write-baseline` how many of the entries it wrote are fixable.

- **`exclude:` now applies to every run.** A rule's hits in excluded files are dropped, in full runs too, and an allow there suppresses nothing; before, `exclude:` only decided whether a narrowed run ran the rule.
- **`override:` changes a pack rule's paths.** `override: {rust/no-unwrap: {exclude: '^build\.rs$'}}` replaces the rule's `files:` or `exclude:` with the repo's; `--explain` marks them `(override)`.
- **Parameters.** A rule's `params:` names values its command and words use as `{name}`, and `override:` sets them for a pack rule. `rust/weasel-type-names` takes `role-words`, the role words it looks for.
- **`necro:rust-style`**, an opt-in pack beside `necro:rust`: `doc-missing` (a `///` on every public item), `module-doc` (a `//!` opening every file), `doc-length` (doc blocks within `header`, 10, and `item`, 6, lines), `comment-border` (`min-dashes`, 6) and `weasel-type-names`. It needs no cargo.
- **`rust/weasel-type-names` is now `rust-style/weasel-type-names`.** Include `necro:rust-style` to keep it, and rename it in allows, `disable:` and the baseline.
- **`necro rust-docs`** finds doc-comment problems in tracked `.rs` files (`missing`, `module`, `length`) and, like `necro grep`, searches only the given files in a narrowed run.
- **`necro cargo-manifest --edition <year>`** sets the edition members must be on; `rust/cargo-manifest` takes it as the `edition` parameter (default 2024).
- **`necro --list`** prints every rule, where it comes from, and whether `necro.yaml` disables or overrides it.

## 0.4.0 - 2026-11-05

- Narrowed runs: `necro [<file>…]` checks what concerns the given files, the way a pre-commit hook does. A rule with the new `files:` or `exclude:` keys runs only when a given file passes them, and then runs in full; `necro grep` searches only the given files. Stale baseline entries and allow checks are judged only in those files. The published hook now passes file names; `necro` with no files, and `--write-baseline`, still cover everything.
- `files:` and `exclude:` take prek's form: a regex string, or `{glob: <glob or list>}`. The `necro:rust` rules carry them, so a change without Rust files skips them.
- `necro grep` accepts lookaround and backreferences (it uses `fancy_regex`), so a match can exclude what comes before it without starting early: `(?<![.\w])randf\(`. Such patterns read every file rather than only those `git grep` picks, and a line that exhausts the backtracking limit is an error naming it.
- `necro grep --glob <glob>` picks files by glob, repeatable, with `!` to exclude, read as prek reads globs: against the whole path, with `*` crossing folders. Globs combine with pathspecs.
- `necro grep --files-from <file>` keeps only the listed files; narrowed runs pass it.
- `--explain` shows a rule's `files:` and `exclude:`.
- `necro grep` is several times faster on large repos with the same hits: `git grep -F` picks the files holding text every match must contain, and necro matches them on every core. On rust-lang/rust (64k files) a whole-tree search went from about 2 s to 0.4 s.

## 0.3.0 - 2026-11-01

- README lists `uv tool`, `pip` and `cargo` installs, and its pre-commit snippet no longer pins a version range.
- Packs: `include: [necro:rust]` adds rules built into necro, named `<pack>/<rule>` in output, allows, the baseline, `--rule` and `--explain`, which also names the pack; `disable: [<pack>/<rule>]` turns one off. A pack is versioned with necro, so an upgrade can bring new rules or new hits. The first is `necro:rust`: no-unwrap, no-expect, lint-override-reason, no-stubs, unsafe-safety-comment, swallowed-error, weasel-type-names, lossy-cast and cargo-manifest, mostly clippy lints with necro's wording. necronomicon's own Rust rules now come from it.
- Two Cargo finders, for every Cargo workspace under the root: `necro clippy <lint>…` prints clippy lints as hits, checking each crate's default features, and `necro cargo-manifest` reports crates below edition 2024 or publishable without a `rust-version`.
- `necro clippy` builds in its own `target/necro`, so it never invalidates the repo's own clippy cache, and builds one cold workspace at a time per machine, so a first run takes one build's memory. `necro --clean-cache` removes those folders.
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
