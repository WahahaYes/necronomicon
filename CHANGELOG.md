# Changelog

## Unreleased

- **Options parse the same way in the CLI and every finder.** A value option takes `--name=value` as well as `--name value` (`agent-files` and `rustfmt` rejected `--only-at=…`, `clippy` rejected `--match=…`), options may come anywhere before `--` (`grep` took `--write` only before its regex), and an option a command doesn't know is always `unknown option`.
- **`agent-files/skill-frontmatter`** reads a `SKILL.md`'s frontmatter as YAML and reports frontmatter that doesn't parse or isn't one mapping, rather than half-reading it line by line.
- **`gdscan`** no longer loses its place after a bracket inside a triple-quoted string, so `spacing` and `inline-lambda` keep finding later hits.
- **`necro grep --skip-comments`** keeps a comment marker inside a `"""` string, even one spanning lines, so a `#` in GDScript text isn't read as a comment.
- **`necro rust-docs`** uses a single stateful scanner that tracks block comments, strings, raw strings and character literals across lines, so brackets inside them no longer skew the `cfg(test)` module depth.

## 0.5.1 - 2026-11-06

- **`rust/clippy`** in `necro:rust`: every other warning rustc and clippy give, tests included, labelled with its lint, from the clippy build the other rules already share. Its `skip` parameter drops lints by name; lints the crate's `[lints]` tables turn on show up too. Repos can drop a `cargo clippy -- -D warnings` hook and its second build. `necro clippy --others [--except=…]` is its finder mode, and necro's clippy run now caps lint levels at warn, so a denied lint is a hit rather than a check error.
- **`rust/rustfmt`**, a fixable rule in `necro:rust`: a file `cargo fmt --check` would change is a hit, and `necro --fix` runs rustfmt on it (allowed and baselined files stay as they are). Its finder is `necro rustfmt`. Repos can drop their own `cargo fmt` hook; one that doesn't use rustfmt adds `disable: [rust/rustfmt]`.
- **`gdscript-style/spacing` is fixable.** `necro --fix` joins a top-level comment that blank lines keep from its function (gdformat would detach a `##` doc there), then makes two blank lines above the block; `necro gdscan spacing --write` does it directly. Repos can drop their own spacing fixer.
- **`gdscript/inline-lambda` flags any lambda inside brackets**, including one in a one-line call (`x.filter(func(p): ...)`), since gdformat may wrap that call across lines and corrupt it; `necro gdtoolkit format --write` skips those files too. Name such a lambda first: `var keep: Callable = func(p): ...`.

## 0.5.0 - 2026-11-06

- Finders print hits sorted by line and column as numbers; `necro gdtoolkit` sorted them as text, so line 325 came before line 74.
- `necro gdtoolkit lint` reports `max-returns` at its function's `func` line instead of the last `return`, so baseline entries fingerprint the function.
- A repeated hit's label follows a colon (`also at src/a.gd:2:1: (.shuffle()`) instead of parentheses, which read as unbalanced around a match ending in a bracket.
- Allows aren't read from the files rules come from (`necro.yaml`, its includes and `{file: …}` texts), so a rule's words can quote the allow syntax without tripping `necro/allow-reason` or `necro/unused-allow`.
- **`necro:gdscript`**, a pack for Godot: `gdformat` (fixable with `--fix`, parameter `line-length`), `gdlint`, `parse`, `inline-lambda` (a lambda inside a multi-line bracket, which gdformat corrupts and `--fix` skips) and `uid` (each script and its `.gd.uid` tracked together, once a repo tracks any). It runs gdtoolkit from `PATH`: install `necronomicon[gdscript]`, which pins gdtoolkit 4.5.0 beside necro.
- **`necro:gdscript-style`**, opt-in GDScript style: `walrus`, `border` (`min-dashes`), `spacing`, `export-doc` and `doc-length` (`header` 10, `item` 6). It needs no gdtoolkit.
- **`necro gdscan`** and **`necro gdtoolkit`**, the finders behind them; like `necro grep`, they search only the given files in a narrowed run.
- **`necro:agent-files`**, a pack for the files coding agents read: `claude-md` (a `CLAUDE.md` of only `@AGENTS.md` beside each `AGENTS.md`, fixable), `claude-md-alone`, `skill-link` (each skill in `.agents/skills` linked from `.claude/skills`, fixable), `skill-misplaced`, `skill-frontmatter`, `other-instructions`, `rule-reference` (a rule named in agent instructions that the repo doesn't have) and `stale-skill`. Its finder is `necro agent-files`.
- **Skills for coding agents.** `necro skills install` writes the `necro` skill (working with hits, allows and the baseline, reporting necro bugs) and `necro-rules` (writing rules, keeping `AGENTS.md` in step) into `.agents/skills`, stamped with this version and linked from `.claude/skills`; `agent-files/stale-skill` fixes with it, so `necro --fix` refreshes them after an upgrade. `necro-adopt`, for setting necro up, installs with `npx skills add WahahaYes/necronomicon --skill necro-adopt`.
- Issue forms for bugs and pack suggestions, which the skills point to.
- **A top-level `exclude:`** leaves files out of the whole run: no rule's hits there, no allows read there, and a narrowed run drops them. For vendored code, fixtures, and files that quote allows as data.
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
