# Writing rules

A rule is a command that prints where the violations are, plus the words necro shows whoever trips it. The full contract is [spec/v0.md](../spec/v0.md); [examples/](../examples/) holds small working repos.

## The command

necro runs it from the repo root, splits it into words like a shell (quotes) but runs no shell, and reads one hit per output line: `path[:line[:col[-endcol]]][:label]`. Exit 0 or 1 is normal. It may use git and the repo's own toolchain, nothing else.

- **A pattern:** `necro grep [--skip-comments <marker>] '<regex>' -- <pathspecs>`. Rust regex, columns right for every match, the match as the label. Exclusions are pathspecs (`':!src/net/protocol.gd'`).
- **More than a pattern:** a script in the repo's language that prints the same lines. For Python, `uv run --script necro/x.py` with the script's needs in a PEP 723 header, so nobody installs anything.
- **Any other tool** works if its output fits, `git grep -n --column -o` included (its columns count bytes and drift after the first match on a line).

The label is what the hit is about (`missing steal_roll`, `3 lines over`). It shows beside the hit, and it's part of the hit's identity in the baseline, so keep it stable.

## The words

`message` says what's wrong in one line, `why` why the rule exists, `fix` what to do instead. Write them for someone who has never seen the rule. `details` (inline or `{file: …}`) and `examples` (`bad` and `good` snippets) only show under `necro --explain <rule>`; necro doesn't run the examples.

## Exceptions and adoption

- **Allows:** `necro-allow <rule>: <reason>` in a comment on the hit's line or the one above. A missing reason and an allow that suppresses nothing are errors.
- **The baseline:** `necro baseline` writes `necro.baseline`, the counts a repo accepts while it adopts a rule. Commit it; new hits still fail, and when counts drop, run it again to tighten the ratchet.
- **Trial rules:** `trial: true` shows hits without failing, for a rule still being tuned.

## Layout

`necro.yaml` at the root holds `version: 0` and the rules; `include: [necro/*.yaml]` splits them across files, and scripts live beside them in `necro/`. Run it from pre-commit:

```yaml
- repo: https://github.com/WahahaYes/necronomicon
  rev: <tag or sha>
  hooks: [{ id: necro }]
```
