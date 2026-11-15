# Behaviors

What necro's users rely on, one entry per behavior: a slug, a scenario, the spec section it comes from, and the conformance cases that prove it. Each case in `tests/conformance/cases/` names one entry in its `behavior:` header, and the suite fails when a case names a slug that isn't here or an entry's **Proved by** list differs from the cases naming it.

A bug fix adds a case to the behavior it broke. If no entry fits, add the entry first.

## Allows

### `allow-suppresses-hit`

**Scenario:** Given a hit, when the file has `necro-allow <rule>: <reason>` in a comment on the hit's line or the line above (line 1, or 2 after `#!`, for a whole-file hit), then the hit is suppressed; one comment may carry allows for two rules.

**Spec:** [Allows](../spec/v0.md#allows)

**Proved by:** `allows/suppress.case`, `allows/two-on-a-line.case`, `allows/whole-file.case`

### `allow-needs-a-reason`

**Scenario:** Given an allow with no reason after the rule, when necro runs, then it reports `necro/allow-reason`, and the allow still suppresses.

**Spec:** [Allows](../spec/v0.md#allows)

**Proved by:** `allows/missing-reason.case`

### `unused-allow-fails`

**Scenario:** Given an allow that suppresses nothing or names no rule, when necro runs, then it reports `necro/unused-allow` wherever the allow is.

**Spec:** [Allows](../spec/v0.md#allows)

**Proved by:** `allows/unused.case`

### `quoted-syntax-is-not-an-allow`

**Scenario:** Given text that quotes the allow syntax outside a comment (prose, data, a string) or in the files rules are read from, when necro runs, then it is not an allow and raises nothing.

**Spec:** [Allows](../spec/v0.md#allows)

**Proved by:** `allows/prose.case`, `allows/rule-words.case`

### `allows-found-from-root`

**Scenario:** Given a commit hook in a git worktree, which sets `GIT_DIR` and `GIT_WORK_TREE`, when necro runs, then it still finds the allows from the root.

**Spec:** [Allows](../spec/v0.md#allows)

**Proved by:** `allows/hook-git-env.case`

### `summary-lists-allows`

**Scenario:** Given suppressed hits, when necro prints in full, then it lists each allow with the hit's location and its reason, comment closers trimmed.

**Spec:** [Allows](../spec/v0.md#allows)

**Proved by:** `allows/summary.case`

## Baseline

### `baseline-fingerprint-matching`

**Scenario:** Given a baseline entry, when a hit's source line matches the fingerprint (trimmed, before any allow), then it is silenced; a token change fails and leaves the entry stale; identical lines share one bucket so one entry silences one hit and the excess fails.

**Spec:** [Baseline](../spec/v0.md#baseline)

**Proved by:** `baseline/edited-line.case`, `baseline/over-count.case`

### `baseline-line-is-hint`

**Scenario:** Given a baseline entry with a stored line, when the matching line moves to a different line number, then it is still silenced because the line is only a hint.

**Spec:** [Baseline](../spec/v0.md#baseline)

**Proved by:** `baseline/line-shift.case`

### `baseline-new-hits-fail`

**Scenario:** Given a baseline, when a hit appears that the snapshot doesn't cover, then it fails; a missing snapshot file means no baseline and the run behaves exactly as without one.

**Spec:** [Baseline](../spec/v0.md#baseline)

**Proved by:** `baseline/new-rule.case`, `baseline/missing.case`

### `baseline-scoped-regen-merges`

**Scenario:** Given a baseline with entries for multiple rules, when `necro --write-baseline --rule <name>` runs, then only that rule's entries are replaced and every other rule's entries carry over.

**Spec:** [Baseline](../spec/v0.md#baseline)

**Proved by:** `baseline/scoped-regen-merges.case`

### `baseline-scoped-run-no-stale`

**Scenario:** Given a baseline with entries for multiple rules, when `necro --rule <name>` runs, then only that rule's entries are matched and other rules' leftover entries never warn as stale.

**Spec:** [Baseline](../spec/v0.md#baseline)

**Proved by:** `baseline/rule-filter-no-stale.case`

### `baseline-allows-before-snapshot`

**Scenario:** Given a hit with an allow comment, when necro runs, then the allow applies before the snapshot so the hit never reaches it and the run is clean.

**Spec:** [Baseline](../spec/v0.md#baseline)

**Proved by:** `baseline/suppression-wins.case`

### `baseline-builtins-never-snapshotted`

**Scenario:** Given a broken allow that produces a builtin diagnostic, when `necro --write-baseline` runs, then the builtin is never snapshotted and the broken allow still fails on the next run.

**Spec:** [Baseline](../spec/v0.md#baseline)

**Proved by:** `baseline/regen-skips-builtins.case`, `baseline/builtin-not-silenced.case`

### `baseline-empty-regen-writes-empty`

**Scenario:** Given a clean tree with no hits, when `necro --write-baseline` runs, then it writes an empty snapshot `entries: []` instead of deleting the file.

**Spec:** [Baseline](../spec/v0.md#baseline)

**Proved by:** `baseline/empty-regen.case`

### `baseline-unused-allow-errors`

**Scenario:** Given an allow that suppresses nothing, when necro runs with a baseline covering the named rule, then it still reports `necro/unused-allow` because the baseline covers the named rule, not the builtin.

**Spec:** [Baseline](../spec/v0.md#baseline)

**Proved by:** `baseline/unused-allow.case`

### `baseline-stale-entries-warn`

**Scenario:** Given baseline entries that nothing consumes, when necro runs, then they warn on stderr without failing the run.

**Spec:** [Baseline](../spec/v0.md#baseline)

**Proved by:** `baseline/stale.case`

## Messages

### `messages-progressive-repeats`

**Scenario:** Given multiple hits for the same rule, when necro prints in full, then the first hit teaches with source, underline, label, why, fix and explain; repeats show the location, label and fix; allows are listed in the summary.

**Spec:** [Messages](../spec/v0.md#messages)

**Proved by:** `full/progressive.case`

### `messages-underline-and-label`

**Scenario:** Given a hit with `col-endcol`, when necro prints in full, then the underline spans the columns; a label that isn't the underlined text sits beside the caret on the first hit and after the location on repeats; with only `col` a single caret is shown.

**Spec:** [Messages](../spec/v0.md#messages)

**Proved by:** `full/underline-and-label.case`, `full/label-beside.case`

### `messages-wrap`

**Scenario:** Given a rule with long `why` and `fix` texts, when necro prints in full, then they wrap at 88 columns with continuation lines aligned under the text.

**Spec:** [Messages](../spec/v0.md#messages)

**Proved by:** `full/wrap.case`

### `messages-gutter-and-tabs`

**Scenario:** Given a hit on a line with a high line number or tabs, when necro prints in full, then the gutter widens to fit the line number and tabs show as 4 spaces with the underline moved to match.

**Spec:** [Messages](../spec/v0.md#messages)

**Proved by:** `full/gutter-and-tabs.case`

### `messages-whole-file`

**Scenario:** Given a whole-file hit (no line number), when necro prints in full, then it names the file with the label in parentheses and shows no source line.

**Spec:** [Messages](../spec/v0.md#messages)

**Proved by:** `full/whole-file.case`

### `messages-folded-repeats`

**Scenario:** Given more than 4 repeats of a rule in a file, when necro prints in full, then the first 4 are shown with location and fix and the rest are counted as `N more in <file>`.

**Spec:** [Messages](../spec/v0.md#messages)

**Proved by:** `full/folded.case`

## Skills

### `skills-usage-requires-action`

**Scenario:** Given `necro skills` with no action, when necro runs, then it exits 2 and tells the user to name an action.

**Spec:** [Skills](../spec/v0.md#skills)

**Proved by:** `skills/usage.case`

### `skills-install-writes-and-links`

**Scenario:** Given skills to install, when `necro skills install` runs, then it writes each skill to `.agents/skills/<name>/SKILL.md` stamped with this necro's version, replacing an older copy, and links `.claude/skills/<name>` to `../../.agents/skills/<name>`, replacing a link that points elsewhere.

**Spec:** [Skills](../spec/v0.md#skills)

**Proved by:** `skills/install.case`

### `skills-install-paths-selective`

**Scenario:** Given `SKILL.md` paths as arguments, when `necro skills install` runs, then it rewrites only the carried skills they name and skips the rest.

**Spec:** [Skills](../spec/v0.md#skills)

**Proved by:** `skills/install-paths.case`

### `skills-folder-in-way-stops`

**Scenario:** Given a real folder at `.claude/skills/<name>`, when `necro skills install` runs, then it stops before anything is written and tells the user to move or remove the folder.

**Spec:** [Skills](../spec/v0.md#skills)

**Proved by:** `skills/folder-in-the-way.case`

### `skills-outside-git-fails`

**Scenario:** Given a directory that is not a git work tree, when `necro skills install` runs, then it exits 2 because skills install writes at the top of the git work tree.

**Spec:** [Skills](../spec/v0.md#skills)

**Proved by:** `skills/not-git.case`

## Agent files

### `agent-files-usage-valid-checks`

**Scenario:** Given `necro agent-files --write` with a check that isn't `claude-md` or `skill-link`, when necro runs, then it exits 2 and lists the checks that support `--write`.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `agent-files/usage.case`

### `agent-files-skill-frontmatter-validates`

**Scenario:** Given skills in `.agents/skills`, when `necro agent-files skill-frontmatter` runs, then it validates each `SKILL.md` has `---` frontmatter that is a YAML mapping, a `name` matching the folder (lowercase hyphenated words), and a `description` up to 1024 characters; invalid frontmatter says why.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `agent-files/skill-frontmatter.case`, `agent-files/skill-frontmatter-invalid.case`

### `agent-files-claude-md-checks`

**Scenario:** Given `AGENTS.md` files, when `necro agent-files claude-md` runs, then each `AGENTS.md` must have a `CLAUDE.md` beside it that only imports it (`@AGENTS.md`) or is a symlink to it; a `CLAUDE.md` holding more is reported.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `agent-files/claude-md.case`

### `agent-files-claude-md-alone`

**Scenario:** Given a `CLAUDE.md` with no `AGENTS.md` beside it, when `necro agent-files claude-md-alone` runs, then it reports the `CLAUDE.md` as having no `AGENTS.md` beside it.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `agent-files/claude-md-alone.case`

### `agent-files-skill-link-checks`

**Scenario:** Given skills in `.agents/skills`, when `necro agent-files skill-link` runs, then each skill must have a `.claude/skills/<name>` symlink to `../../.agents/skills/<name>`; a link pointing elsewhere or a missing link is reported on the `SKILL.md`.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `agent-files/skill-link.case`

### `agent-files-rule-reference-checks`

**Scenario:** Given `AGENTS.md`, `CLAUDE.md` and `SKILL.md` files that reference rules via `necro --explain`, `necro --rule` or backticked `<pack>/<rule>`, when `necro agent-files rule-reference` runs, then each referenced rule must be one that `necro.yaml` turns on; disabled pack rules and unknown rules are reported.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `agent-files/rule-reference.case`

### `agent-files-stale-skill-detects`

**Scenario:** Given a skill whose `SKILL.md` frontmatter has `metadata: necro-version:` different from the running necro, when `necro agent-files stale-skill` runs, then it reports the skill as stale.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `agent-files/stale-skill.case`

### `agent-files-other-instructions-detects`

**Scenario:** Given instruction files for other harnesses (`.cursorrules`, `.windsurfrules`, `.clinerules`, `GEMINI.md`, `.github/copilot-instructions.md`), when `necro agent-files other-instructions` runs, then each must mention `AGENTS.md` or it is reported.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `agent-files/other-instructions.case`

### `agent-files-skill-misplaced-detects`

**Scenario:** Given a real folder in `.claude/skills` or a link there to no skill in `.agents/skills`, when `necro agent-files skill-misplaced` runs, then the real folder is reported on its `SKILL.md` and the dangling link is reported on the link.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `agent-files/skill-misplaced.case`

### `agent-files-fix-creates-missing`

**Scenario:** Given a missing `CLAUDE.md` beside an `AGENTS.md` and a skill without a `.claude/skills` link, when `necro --fix` runs with the `necro:agent-files` pack, then it creates the missing `CLAUDE.md` (as `@AGENTS.md`) and the link, then checks again; a `CLAUDE.md` with more content stays for a person.

**Spec:** [Fixes](../spec/v0.md#fixes)

**Proved by:** `agent-files/fix.case`

### `agent-files-fix-updates-stale-skill`

**Scenario:** Given a stale skill that this necro carries and one it doesn't, when `necro --fix` runs with the `necro:agent-files` pack, then it rewrites the carried skill's `necro-version` to the current version and leaves the other alone.

**Spec:** [Packs](../spec/v0.md#packs)

**Proved by:** `agent-files/stale-skill-fix.case`
