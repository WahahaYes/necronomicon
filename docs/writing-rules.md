# Writing rules

A rule is a command that prints where the violations are, plus the words necro shows whoever trips it. The full contract is [spec/v0.md](../spec/v0.md); [examples/](../examples/) holds small working repos.

## The command

necro runs it from the repo root, splits it into words like a shell (quotes) but runs no shell, and reads one hit per output line: `path[:line[:col[-endcol]]][:label]`. Exit 0 or 1 is normal. It may use git and the repo's own toolchain, nothing else.

- **A pattern:** `necro grep [--skip-comments <marker>] [--glob <glob>] '<regex>' -- <pathspecs>`. Rust regex with lookaround (`(?<![.\w])randf\(`), columns right for every match, the match as the label. Pick files with globs as prek reads them, `*` crossing folders (`--glob '*.gd' --glob '!src/net/protocol.gd'`) or pathspecs (`':!src/net/protocol.gd'`). Fast on large repos: `git grep` picks the files holding the regex's fixed text, then necro matches on every core. A pattern with lookaround or backreferences skips that step and reads every file it's given, so narrow it with a pathspec or `--glob`.
- **Rust:** `necro clippy [--tests] <lint>…` reports clippy lints, from the set the Rust pack uses, in every Cargo workspace under the root; `necro cargo-manifest` checks each crate's edition and `rust-version`. Test code is skipped unless `--tests`. `necro clippy` builds in `necro/` inside each target folder; `necro --clean-cache` removes it.
- **More than a pattern:** a script in the repo's language that prints the same lines. For Python, `uv run --script necro/x.py` with the script's needs in a PEP 723 header, so nobody installs anything.
- **Any other tool** works if its output fits, `git grep -n --column -o` included (its columns count bytes and drift after the first match on a line).

The label is what the hit is about (`missing steal_roll`, `3 lines over`). It shows beside the hit.

## The words

`message` says what's wrong in one line, `why` why the rule exists, `fix` what to do instead. Write them for someone who has never seen the rule. `details` (inline or `{file: …}`) and `examples` (`bad` and `good` snippets) only show under `necro --explain <rule>`; necro doesn't run the examples.

## Allows

`necro-allow <rule>: <reason>` in a comment on the hit's line or the one above. A missing reason and an allow that suppresses nothing are errors. A pack rule goes by its full name, `rust/no-unwrap`.

## Packs

`include: [necro:rust]` adds rules built into necro, versioned with it: the Rust pack covers what default clippy leaves off (no `unwrap` or `expect` outside tests, lint overrides as `#[expect]` with a reason, `SAFETY` comments, dropped `Result`s, truncating casts, the crate's edition) and rustfmt's layout, which `--fix` applies. Its rules are named `rust/<rule>`; `necro --explain rust/<rule>` gives each one's words. `necro:rust-style` is the opt-in companion: a `///` on every public item, a `//!` opening every file, doc blocks within a line limit, no comment borders and no role-word type names. For Godot, `necro:gdscript` runs gdtoolkit (gdformat, fixable with `--fix`; gdlint; parse errors) plus guards for a lambda gdformat corrupts and for scripts committed without their `.uid`, and `necro:gdscript-style` adds the opt-in style rules. gdtoolkit comes with the wheel's extra: `necronomicon[gdscript]`. `necro:agent-files` checks the files coding agents read: a `CLAUDE.md` of only `@AGENTS.md` beside each `AGENTS.md`, skills in `.agents/skills` linked from `.claude/skills`, skill frontmatter, and pointers to rules that no longer exist ([agents-and-rules.md](agents-and-rules.md)). `disable: [rust/lossy-cast]` turns a rule off for the repo; for single hits use allows or the baseline. `override:` gives a pack rule the repo's own `files`, `exclude` or [parameters](#parameters), so it can skip paths it shouldn't judge or loosen a limit:

```yaml
override:
  rust/no-unwrap: { exclude: '^build\.rs$' }
```

Every rule of an included pack is on unless `disable` names it; rules a repo opts into ship as their own pack. `necro --list` shows each rule, where it comes from, and what `necro.yaml` changed.

## Parameters

`params:` names values a rule's command and words use, so a rule in a pack or an include can ship a default a repo changes with `override:` instead of copying the rule:

```yaml
doc-length:
  command: uv run --script necro/docs.py --header {header} --item {item}
  params: { header: 10, item: 6 }
  message: a doc block is over its limit
  why: Long docs go stale and hide the code.
  fix: cut it to {header} lines for the file header, {item} for any other doc.
```

Only declared names are replaced, so a regex's own braces (`-{6,}`) stay. A value is one word of the command however it reads, and a list given as a whole word (`-- {paths}`) becomes one word per item.

## Fixes

A rule that knows its fix can apply it. For a pattern, give `necro grep` the replacement; for anything else, name the fixing command in `autofix`, with `{files}` where the files go:

```yaml
trailing-space:
  command: necro grep --replace '' '[ \t]+$' -- '*.gd'
license-header:
  command: uv run --script necro/header.py --check
  autofix: uv run --script necro/header.py --write {files}
```

`necro --fix` (what the hook runs) fixes the hits a run reports, one rule at a time, then checks again: `necro: fixed 3 hits in 2 files (…); review and re-stage them`, followed by any hits it couldn't fix in full. Allowed and baselined hits stay as they are, so run `--fix` before `--write-baseline` to keep fixable hits out of the snapshot.

## Narrowed runs

Given files (pre-commit passes the changed ones), necro skips a rule whose `files:` and `exclude:` take none of them, and `necro grep` searches only them. Give a costly rule a filter, in prek's form: a regex string, or `{glob: …}` with one glob or a list.

```yaml
typecheck:
  command: uv run --script necro/typecheck.py
  files: \.py$
  exclude: {glob: "src/generated/*"}
```

A rule that runs runs in full, so a hit it causes elsewhere still shows, except in excluded files: `exclude` puts files outside the rule in every run, and its hits there are dropped. A rule without a filter always runs. `necro` with no files, and `--write-baseline`, cover everything.

## Layout

`necro.yaml` at the root holds `version: 0` and the rules; `include: [necro/*.yaml]` splits them across files, and scripts live beside them in `necro/`. A top-level `exclude:`, in the same form as a rule's, leaves files out of every rule and every run: vendored code, fixtures, or files that quote rules and allows as data, whose example allows necro then doesn't read. Run it from pre-commit with the hook in the [README](../README.md#install), or, to track a necronomicon clone instead of a release (see `.pre-commit-config.yaml` here):

```yaml
- repo: local
  hooks:
    - id: necro
      name: necro
      entry: cargo run -q --manifest-path <clone>/Cargo.toml --
      language: system
      always_run: true
      require_serial: true
```
