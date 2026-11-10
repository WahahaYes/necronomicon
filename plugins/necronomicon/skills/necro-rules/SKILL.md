---
name: necro-rules
description: Write or change a necro rule in necro.yaml, and keep AGENTS.md in step with it. Use when adding a house rule, when a correction keeps repeating, when an AGENTS.md line could be checked mechanically, or when allows pile up on a rule.
---

<!-- Derived from .agents/skills/necro-rules/SKILL.md: edit the canonical file there, then re-copy here. -->

# Writing necro rules

A convention has one home. If a command can check it, it's a rule, whose words teach whoever trips it; judgment stays in `AGENTS.md` or a skill. [agents-and-rules.md](https://github.com/WahahaYes/necronomicon/blob/main/docs/agents-and-rules.md) says when to move one; [writing-rules.md](https://github.com/WahahaYes/necronomicon/blob/main/docs/writing-rules.md) has the format.

## Writing one

1. Check the packs first: `necro --list` shows what's on, and a pack rule may already cover it.
2. The command prints `path:line:col` per hit: `necro grep '<regex>' -- '<pathspec>'` for a pattern, a script (`uv run --script necro/<name>.py`) for more, or a tool the repo already runs.
3. The words: `message` says what's wrong in one line; `why` gives the reason, so a reader can judge an edge case; `fix` says what to do instead, concretely. Add `examples` (`bad`, `good`) and `details` for `--explain`.
4. Give it `files:` (prek's form: a regex, or `{glob: …}`) so a commit that touches none of them skips it.
5. Try it over the whole repo: `necro --rule <name>`, and read a few hits. Fix the real ones; for the rest, suggest the user baselines them (`necro --write-baseline --rule <name>`), which is theirs to run.

## Keeping AGENTS.md in step

- A rule that shapes the architecture gets a one-line pointer in `AGENTS.md`, naming it: "core never imports app; necro checks it (`core-isolated`)".
- An `AGENTS.md` line that said the same thing shrinks to that pointer, or goes.
- Allows piling up on a rule mean the rule is wrong: narrow it, or move the convention back to prose.

## Suggesting it for a pack

If the rule would suit any repo in that language, offer to suggest it for necro's packs with the [pack suggestion form](https://github.com/WahahaYes/necronomicon/issues/new?template=pack-suggestion.yml): the rule's YAML, the language, and its hit count here. Ask the user before filing.
