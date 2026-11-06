---
name: necro
description: Work in a repo whose house rules necro checks (a necro.yaml at the top). Use when a commit or hook prints necro hits, before committing in such a repo, or when necro itself crashes or reports something wrong.
---

# Working with necro

necro checks this repo against its house rules in `necro.yaml`, usually as a pre-commit hook. Each hit names its rule and says what's wrong, why, and what to do instead.

## When a commit fails

1. Read the hit: the rule, the line, `why` and `fix`. `necro --explain <rule>` gives the long form and examples.
2. If the hook ran `necro --fix`, it already changed files: review them and stage them again. Run `necro --fix` yourself for fixable hits it didn't reach.
3. Otherwise change the code as `fix` says.
4. Only when the rule is wrong for this one line, add an allow: a comment `necro-allow <rule>: <reason>` on the line or the one above, with a reason a reviewer would accept. Allows without a reason, or that suppress nothing, fail too.
5. Never run `necro --write-baseline` to get a commit through. The baseline holds known hits; refreshing it is the user's call.

Exit 2 is a config or setup error, not a hit: fix what it names.

## Before committing

Run `necro <changed files>`, or `necro` for everything, and fix what it reports. `necro --list` shows every rule.

## When a rule surprises you

A rule you didn't see coming means `AGENTS.md` is missing a line. When you finish, tell the user which rule tripped and the line that would have warned you.

## When necro itself is wrong

necro crashed, reported a hit that's wrong, missed one, or worded one confusingly. Draft an issue for necronomicon with its bug form's headings, as `###` headings: `What happened` (the command and necro's output as printed), `What you expected`, `necro --version`, and `Repro` (a `necro.yaml` and a file or two that show it, none of this repo's code). Show the draft and ask the user; only if they agree, file it with `gh issue create --repo WahahaYes/necronomicon --label bug --title <title> --body <draft>`.
