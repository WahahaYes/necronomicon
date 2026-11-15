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

## Commands

### `command-errors-are-check-errors`

**Scenario:** Given a command that exits other than 0 or 1, prints a line that doesn't parse, or names a path that isn't a file, when necro runs, then it reports a check error with the end of the command's stderr, and other rules still report.

**Spec:** [Commands](../spec/v0.md#commands)

**Proved by:** `command/bad-line.case`, `command/exit-code.case`, `command/not-a-file.case`

### `clean-command-run-is-silent`

**Scenario:** Given a command whose exit 1 means no match, when necro runs and finds nothing, then it prints nothing.

**Spec:** [Commands](../spec/v0.md#commands)

**Proved by:** `command/clean.case`

### `command-split-without-a-shell`

**Scenario:** Given a command, when necro reads it, then it is split into POSIX shell words with quotes and backslashes but run directly, so variables and other expansions stay as written.

**Spec:** [Commands](../spec/v0.md#commands)

**Proved by:** `command/words.case`

### `command-runs-at-the-root`

**Scenario:** Given `-C <dir>` or git's `GIT_DIR` and `GIT_WORK_TREE`, when a command runs, then it runs in the nearest folder at or above the start that holds `necro.yaml`, with git finding the repo from there.

**Spec:** [Commands](../spec/v0.md#commands)

**Proved by:** `command/hook-git-env.case`, `command/root-dir.case`

### `no-config-is-a-setup-error`

**Scenario:** Given no `necro.yaml` at or above the start, when necro runs, then it reports a setup error naming the folder.

**Spec:** [Commands](../spec/v0.md#commands)

**Proved by:** `command/no-root.case`

### `command-finds-necro-on-path`

**Scenario:** Given a rule that calls `necro grep`, when necro runs it with a `PATH` that doesn't have necro installed, then the command still finds the same necro first on its `PATH`.

**Spec:** [Commands](../spec/v0.md#commands)

**Proved by:** `command/finder-path.case`

### `missing-command-is-a-setup-error`

**Scenario:** Given a command whose program isn't on `PATH`, when necro runs it, then it reports a setup error naming the rule and the program.

**Spec:** [Commands](../spec/v0.md#commands)

**Proved by:** `command/missing-program.case`

### `command-hits-in-any-form`

**Scenario:** Given a command's non-empty stdout lines in any of `path[:line[:col[-endcol]]][:label]` forms, when necro prints a terse run, then each line is one hit, from any command including `git grep`, sorted by rule, path, line and column.

**Spec:** [Commands](../spec/v0.md#commands)

**Proved by:** `command/git-grep.case`, `command/output-forms.case`

## Narrowed runs

### `files-and-exclude-choose-what-a-rule-checks`

**Scenario:** Given files, when a rule has `files` or `exclude`, then it runs in full when any given file passes both (a glob matches the whole path, `*` crossing folders) and is skipped otherwise, while a rule with only `exclude` takes every file it doesn't exclude and one with neither always runs.

**Spec:** [Narrowed runs](../spec/v0.md#narrowed-runs)

**Proved by:** `narrow/exclude.case`, `narrow/filters.case`, `narrow/globs.case`, `narrow/sync-skills-files.case`

### `files-and-exclude-take-a-regex-or-glob`

**Scenario:** Given a rule's `files` or `exclude`, when it is a regex string or `{glob: <glob or list>}`, then the config loads, and anything else is a config error naming the key and its position.

**Spec:** [Narrowed runs](../spec/v0.md#narrowed-runs)

**Proved by:** `narrow/bad-filters.case`

### `narrowed-paths-reach-the-root`

**Scenario:** Given file arguments relative to the working folder, when necro runs, then it re-anchors them to the root and drops ones that aren't files or that the top-level `exclude` leaves out.

**Spec:** [Narrowed runs](../spec/v0.md#narrowed-runs)

**Proved by:** `narrow/reanchor.case`, `narrow/top-exclude.case`

### `finders-take-the-given-files`

**Scenario:** Given files, when necro's file finders such as `necro grep` and `necro rust-docs` run in a narrowed run, then they search only those files.

**Spec:** [Narrowed runs](../spec/v0.md#narrowed-runs)

**Proved by:** `narrow/finder.case`, `narrow/rust-docs.case`

### `allows-and-baseline-count-only-given-files`

**Scenario:** Given files, when necro judges allows and stale baseline entries, then it counts only those inside the given files.

**Spec:** [Narrowed runs](../spec/v0.md#narrowed-runs)

**Proved by:** `narrow/allows.case`, `narrow/stale.case`

### `unscoped-flags-refuse-files`

**Scenario:** Given file arguments, when necro is asked to `--explain`, `--list` or `--write-baseline`, then it is a usage error telling the caller to drop them.

**Spec:** [Narrowed runs](../spec/v0.md#narrowed-runs)

**Proved by:** `narrow/refused-list.case`, `narrow/refused.case`, `narrow/write-baseline.case`
