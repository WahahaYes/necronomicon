# Writing rules

A rule is a command that prints where the violations are, plus the words necro shows whoever trips it. The full contract is [spec/v0.md](../spec/v0.md); [examples/](../examples/) holds small working repos.

## The command

necro runs it from the repo root, splits it into words like a shell (quotes) but runs no shell, and reads one hit per output line: `path[:line[:col[-endcol]]][:label]`. Exit 0 or 1 is normal. It may use git and the repo's own toolchain, nothing else.

- **A pattern:** `necro grep [--skip-comments <marker>] '<regex>' -- <pathspecs>`. Rust regex, columns right for every match, the match as the label. Exclusions are pathspecs (`':!src/net/protocol.gd'`).
- **Rust:** `necro clippy [--tests] <lint>…` reports clippy lints, from the set the Rust pack uses, in every Cargo workspace under the root; `necro cargo-manifest` checks each crate's edition and `rust-version`. Test code is skipped unless `--tests`. `necro clippy` builds in `necro/` inside each target folder; `necro --clean-cache` removes it.
- **More than a pattern:** a script in the repo's language that prints the same lines. For Python, `uv run --script necro/x.py` with the script's needs in a PEP 723 header, so nobody installs anything.
- **Any other tool** works if its output fits, `git grep -n --column -o` included (its columns count bytes and drift after the first match on a line).

The label is what the hit is about (`missing steal_roll`, `3 lines over`). It shows beside the hit.

## The words

`message` says what's wrong in one line, `why` why the rule exists, `fix` what to do instead. Write them for someone who has never seen the rule. `details` (inline or `{file: …}`) and `examples` (`bad` and `good` snippets) only show under `necro --explain <rule>`; necro doesn't run the examples.

## Allows

`necro-allow <rule>: <reason>` in a comment on the hit's line or the one above. A missing reason and an allow that suppresses nothing are errors. A pack rule goes by its full name, `rust/no-unwrap`.

## Packs

`include: [necro:rust]` adds rules built into necro, versioned with it: the Rust pack covers what default clippy leaves off (no `unwrap` or `expect` outside tests, lint overrides as `#[expect]` with a reason, `SAFETY` comments, dropped `Result`s, truncating casts, the crate's edition). Its rules are named `rust/<rule>`; `necro --explain rust/<rule>` gives each one's words. `disable: [rust/lossy-cast]` turns a rule off for the repo; for single hits use allows or the baseline.

## Layout

`necro.yaml` at the root holds `version: 0` and the rules; `include: [necro/*.yaml]` splits them across files, and scripts live beside them in `necro/`. Run it from pre-commit with the hook in the [README](../README.md#install), or, to track a necronomicon clone instead of a release (see `.pre-commit-config.yaml` here):

```yaml
- repo: local
  hooks:
    - id: necro
      name: necro
      entry: sh -c 'cd <clone> && exec cargo run -q -- -C <repo>'
      language: system
      pass_filenames: false
```
