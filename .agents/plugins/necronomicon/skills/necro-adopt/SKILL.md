---
name: necro-adopt
description: Set up necro (necronomicon) in a repo, house rules run as pre-commit checks whose hits explain themselves to coding agents. Use when asked to add necro or necronomicon to a repo, or to turn a repo's lint scripts or AGENTS.md conventions into necro rules.
---

<!-- Derived from .agents/skills/necro-adopt/SKILL.md: edit the canonical file there, then re-copy here. -->

# Adopting necro

## 1. Install

`uv tool install necronomicon` or `pip install necronomicon` (a wheel around the binary), or `cargo install necronomicon`. For Godot, install `necronomicon[gdscript]` instead, which pins gdtoolkit beside the binary. Check with `necro --version`.

## 2. necro.yaml

At the repo top: `version: 0`, then `include:` the packs for what the repo holds: `necro:rust` (and the opt-in `necro:rust-style`) for Rust, `necro:gdscript` (and the opt-in `necro:gdscript-style`) for Godot, `necro:agent-files` when it has `AGENTS.md` or skills. If its main language has no pack, tell the user; they can ask for one with the [pack suggestion form](https://github.com/WahahaYes/necronomicon/issues/new?template=pack-suggestion.yml). A top-level `exclude:` leaves out vendored code and fixtures.

## 3. Rules from what's there

Sort the repo's existing conventions, from `AGENTS.md`, contributing docs and lint scripts: what a command can check becomes a rule (the `necro-rules` skill, once installed in step 6), and its `AGENTS.md` line shrinks to a pointer; judgment stays prose. [agents-and-rules.md](https://github.com/WahahaYes/necronomicon/blob/main/docs/agents-and-rules.md) covers the split.

## 4. First run

Run `necro`. `necro --fix` clears fixable hits; fix the real ones that are quick. Leave the rest to the baseline: ask the user to run `necro --write-baseline`, which records today's hits so only new ones fail. Never baseline without asking.

## 5. Hook and CI

Add the hook from the [README](https://github.com/WahahaYes/necronomicon#install), for prek or pre-commit, whichever the repo uses. It floats to the latest `necronomicon` on fresh hook environments, which is how rule fixes arrive; any 0.x may break the format, so to pin instead name the minor last upgraded to (`necronomicon==0.<minor>.*`). In CI, run `necro` with no files, or `prek run --from-ref origin/main --to-ref HEAD` (`pre-commit run` takes the same arguments).

## 6. Skills

Run `necro skills install`. It writes the `necro` and `necro-rules` skills into `.agents/skills`, stamped with this necro's version, and links them from `.claude/skills`. Commit them. After a necro upgrade, `necro --fix` rewrites them.
