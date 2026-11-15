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

## Config

### `config-rule-name-format`

**Scenario:** Given a rule name that is not lowercase words joined by hyphens, when necro reads the config, then it reports a config error at the rule name.

**Spec:** [A rule](../spec/v0.md#a-rule)

**Proved by:** `config/bad-name.case`

### `config-rule-examples-complete`

**Scenario:** Given a rule whose examples provide only bad or only good snippets, when necro reads the config, then it reports a config error at the missing examples key.

**Spec:** [A rule](../spec/v0.md#a-rule)

**Proved by:** `config/examples-need-both.case`

### `config-rule-exclude-drops-hits`

**Scenario:** Given a rule with an exclude pattern, when necro runs, then hits in excluded files are dropped and allows there are reported as unused.

**Spec:** [A rule](../spec/v0.md#a-rule)

**Proved by:** `config/exclude-drops-hits.case`

### `config-include-merges-rules`

**Scenario:** Given include globs in necro.yaml, when necro reads the config, then rules from matching files are merged, duplicate names are rejected, and included files may only contain rules.

**Spec:** [A rule](../spec/v0.md#a-rule)

**Proved by:** `config/include.case`, `config/include-duplicate.case`, `config/include-only-rules.case`

### `config-required-fields-checked`

**Scenario:** Given a rule missing a required key, when necro reads the config, then it reports a config error pointing at the rule.

**Spec:** [A rule](../spec/v0.md#a-rule)

**Proved by:** `config/missing-required.case`

### `config-git-worktree-required`

**Scenario:** Given a root that is not inside a git work tree, when necro starts, then it reports a setup error.

**Spec:** [Commands](../spec/v0.md#commands)

**Proved by:** `config/not-git.case`

### `config-parameters-format`

**Scenario:** Given a parameter whose name is not lowercase words joined by hyphens or whose value is not a string, number, boolean or list of them, when necro reads the config, then it reports a config error at the parameter.

**Spec:** [Parameters](../spec/v0.md#parameters)

**Proved by:** `config/params-bad.case`

### `config-parameters-substitute`

**Scenario:** Given params in a rule, when necro runs, then `{name}` in the command and words is replaced with the value, a list expands to one word per item in the command and joins with commas elsewhere, and other braces stay as written.

**Spec:** [Parameters](../spec/v0.md#parameters)

**Proved by:** `config/params.case`

### `config-text-file-resolved`

**Scenario:** Given a text field that points at a file, when necro runs, then the file's content is printed as is; when the file does not exist, necro reports a config error.

**Spec:** [A rule](../spec/v0.md#a-rule)

**Proved by:** `config/text-from-file.case`, `config/text-file-missing.case`

### `config-top-exclude-scope`

**Scenario:** Given a top-level exclude, when necro runs, then the excluded files are left out of the whole run: no rule's hits there, no allows read there, and the exclude takes the same forms as a rule's exclude.

**Spec:** [A rule](../spec/v0.md#a-rule)

**Proved by:** `config/top-exclude.case`, `config/top-exclude-bad.case`

### `config-unknown-keys-rejected`

**Scenario:** Given unknown keys in necro.yaml or a rule, when necro reads the config, then each unknown key is reported as a config error in order.

**Spec:** [A rule](../spec/v0.md#a-rule)

**Proved by:** `config/unknown-key.case`

### `config-version-required`

**Scenario:** Given a necro.yaml without a version key, when necro reads the config, then it reports a config error at the missing version.

**Spec:** [Versioning](../spec/v0.md#versioning)

**Proved by:** `config/version-missing.case`

### `config-version-supported`

**Scenario:** Given a necro.yaml with a spec version the engine doesn't implement, when necro reads the config, then it reports a config error naming both versions.

**Spec:** [Versioning](../spec/v0.md#versioning)

**Proved by:** `config/version-newer.case`

### `config-yaml-syntax-errors`

**Scenario:** Given a YAML syntax error in necro.yaml, when necro reads the config, then it reports a config error at the error position with no key path.

**Spec:** [Messages](../spec/v0.md#messages)

**Proved by:** `config/yaml-syntax.case`

## CLI

### `cli-clean-cache-reports-empty`

**Scenario:** Given no cached build folders, when necro runs with `--clean-cache`, then it reports that there is no cache to clean.

**Spec:** [CLI](../spec/v0.md#cli)

**Proved by:** `cli/clean-cache-nothing.case`

### `cli-clean-cache-removes-build`

**Scenario:** Given cached Cargo finders' build folders in workspaces, when necro runs with `--clean-cache`, then it removes each `necro/` build folder and reports its size.

**Spec:** [CLI](../spec/v0.md#cli)

**Proved by:** `cli/clean-cache.case`

### `cli-list-rules`

**Scenario:** Given a config with repo rules, includes, and pack rules with disables and overrides, when necro runs with `--list`, then it prints every rule by name, its source, and for pack rules whether it is disabled or overridden.

**Spec:** [CLI](../spec/v0.md#cli)

**Proved by:** `cli/list.case`

### `cli-rule-filter-runs-one`

**Scenario:** Given `--rule <name>`, when necro runs, then only that rule runs and allows for rules that didn't run are not reported as unused.

**Spec:** [CLI](../spec/v0.md#cli)

**Proved by:** `cli/rule-filter.case`

### `cli-rule-filter-validates-name`

**Scenario:** Given `--rule` with a name that is not a rule, when necro runs, then it exits with a usage error listing the available rules.

**Spec:** [CLI](../spec/v0.md#cli)

**Proved by:** `cli/rule-unknown.case`

### `cli-unknown-option-error`

**Scenario:** Given an unknown command-line option, when necro runs, then it exits with a usage error.

**Spec:** [CLI](../spec/v0.md#cli)

**Proved by:** `cli/unknown-option.case`

### `cli-version-prints`

**Scenario:** Given `--version`, when necro runs, then it prints the engine version.

**Spec:** [CLI](../spec/v0.md#cli)

**Proved by:** `cli/version.case`

## Explain

### `explain-rule-details`

**Scenario:** Given `--explain <rule>`, when necro runs, then it prints the rule's message, why, fix, details, examples, and command with files, exclude and parameters marked when overridden.

**Spec:** [CLI](../spec/v0.md#cli)

**Proved by:** `explain/rule.case`

### `explain-unknown-error`

**Scenario:** Given `--explain` with a name that is not a rule, when necro runs, then it exits with a usage error listing the available rules.

**Spec:** [CLI](../spec/v0.md#cli)

**Proved by:** `explain/unknown.case`

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

## Header

### `header-similar-range`

**Scenario:** Given a `--similar` value outside 1 to 100, when necro header runs, then it reports a usage error naming the invalid value.

**Spec:** [Finders](../spec/v0.md#necro-header)

**Proved by:** `header/bad-similar.case`

### `header-check-finds-missing`

**Scenario:** Given files with missing or wrong headers, when necro header runs, then it reports hits at line 1 (or line 2 after `#!`), and nothing after the header is checked.

**Spec:** [Finders](../spec/v0.md#necro-header)

**Proved by:** `header/check.case`

### `header-fix-writes-header`

**Scenario:** Given a file without a header, when necro header runs with `--fix`, then it writes the header with a blank line after it unless one follows, after any `#!` line.

**Spec:** [Fixes](../spec/v0.md#fixes)

**Proved by:** `header/fix-no-similar.case`

### `header-fix-replaces-old`

**Scenario:** Given a file with an old header recognized by `--similar`, when necro header runs with `--fix`, then it replaces the old header with the new one, keeping a blank line after unless one follows.

**Spec:** [Fixes](../spec/v0.md#fixes)

**Proved by:** `header/fix.case`

### `header-custom-marker`

**Scenario:** Given a file extension without a default marker or with a custom one, when necro header runs with `--marker`, then it uses the given marker for that extension.

**Spec:** [Finders](../spec/v0.md#necro-header)

**Proved by:** `header/marker.case`

### `header-requires-template-or-lines`

**Scenario:** Given no header source, when necro header runs, then it reports a usage error; given both `--template` and line words, it also reports a usage error.

**Spec:** [Finders](../spec/v0.md#necro-header)

**Proved by:** `header/no-header.case`, `header/usage.case`

### `header-params-fill-lines`

**Scenario:** Given a list parameter used as the header lines, when necro header runs, then the parameter's items fill the header one line each.

**Spec:** [Parameters](../spec/v0.md#parameters)

**Proved by:** `header/params.case`

### `header-similar-recognizes-old`

**Scenario:** Given an old header at the expected place with enough similarity, when necro header runs with `--similar`, then it reports `old header, <n>% alike` instead of `no header`.

**Spec:** [Finders](../spec/v0.md#necro-header)

**Proved by:** `header/similar.case`

### `header-template-reads-file`

**Scenario:** Given a `--template` file, when necro header runs, then it reads the header lines from it, drops trailing blank lines, and treats a blank line in the file as the comment marker alone.

**Spec:** [Finders](../spec/v0.md#necro-header)

**Proved by:** `header/template.case`

### `header-unknown-extension-error`

**Scenario:** Given a file whose extension has no known comment marker and no `--marker` override, when necro header runs, then it reports a usage error naming the file.

**Spec:** [Finders](../spec/v0.md#necro-header)

**Proved by:** `header/unknown-extension.case`

## Grep

### `grep-finds-every-match`

**Scenario:** Given files with matches, when necro grep runs, then every non-empty match on a line is a hit with columns in code points, sorted by path, line and column.

**Spec:** [Finders](../spec/v0.md#necro-grep)

**Proved by:** `grep/columns.case`

### `grep-nothing-is-success`

**Scenario:** Given files with no match, when necro grep runs, then it exits 0 with no output.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `grep/nothing.case`

### `grep-regex-alternation-and-lookaround`

**Scenario:** Given a regex with alternation, case folding, lookaround or no fixed text, when necro grep runs, then it finds every match each file holds.

**Spec:** [Finders](../spec/v0.md#necro-grep)

**Proved by:** `grep/alternation.case`, `grep/lookaround-files.case`, `grep/lookbehind.case`, `grep/no-literal.case`

### `grep-globs-narrow-files`

**Scenario:** Given `--glob`s, when necro grep runs, then it keeps files matching the whole-path globs (`*` crossing `/`, a leading `!` excluding) that also pass the pathspecs.

**Spec:** [Finders](../spec/v0.md#necro-grep)

**Proved by:** `grep/globs.case`, `grep/glob-pathspecs.case`

### `grep-pathspecs-read-as-git-grep`

**Scenario:** Given pathspecs, when necro grep runs, then it reads them as git grep does, a leading wildcard excluding as expected.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `grep/pathspecs.case`

### `grep-skip-comments`

**Scenario:** Given `--skip-comments` markers, when necro grep runs, then it ignores the rest of a line from the first marker outside a string, a triple-quoted string may span lines, and a file whose only match is a comment has no hit.

**Spec:** [Finders](../spec/v0.md#necro-grep)

**Proved by:** `grep/skip-comments.case`, `grep/skip-comments-string.case`, `grep/comment-only.case`

### `grep-checks-untracked-files`

**Scenario:** Given untracked files git doesn't ignore, when necro grep runs, then it searches them before they are staged.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `grep/untracked.case`

### `grep-usage-errors`

**Scenario:** Given a missing regex, an invalid regex or an invalid glob, when necro grep runs, then it reports a usage error and exits 2.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `grep/usage.case`, `grep/bad-regex.case`, `grep/bad-glob.case`

## Fixes

### `fix-fixes-reported-hits`

**Scenario:** Given fixable hits, when necro --fix runs, then it fixes the hits the run reports, leaves allowed hits alone, checks again and says what it changed and what is left.

**Spec:** [Fixes](../spec/v0.md#fixes)

**Proved by:** `fix/fix.case`

### `fix-leaves-baselined-hits`

**Scenario:** Given a hit covered by the baseline, when necro --fix runs, then it leaves that hit alone.

**Spec:** [Fixes](../spec/v0.md#fixes)

**Proved by:** `fix/baselined.case`

### `fix-autofix-fills-files`

**Scenario:** Given a rule whose command is not a fixing finder and whose `autofix` uses `{files}`, when necro --fix runs, then `{files}` becomes the files with hits to fix.

**Spec:** [Fixes](../spec/v0.md#fixes)

**Proved by:** `fix/autofix.case`, `fix/sync-skills.case`

### `fix-autofix-needs-files-word`

**Scenario:** Given an `autofix` with no `{files}` word or a parameter named `files`, when necro reads the config, then it reports config errors.

**Spec:** [Fixes](../spec/v0.md#fixes)

**Proved by:** `fix/autofix-config.case`

### `fix-hints-at-fixable-hits`

**Scenario:** Given a full run without `--fix` whose hits a fix could clear, when necro prints, then it ends by saying how many hits can be fixed automatically.

**Spec:** [Fixes](../spec/v0.md#fixes)

**Proved by:** `fix/hint.case`

### `fix-needs-a-check-run`

**Scenario:** Given `--fix` with `--explain`, when necro runs, then it refuses with a usage error.

**Spec:** [Fixes](../spec/v0.md#fixes)

**Proved by:** `fix/refused.case`

### `replace-makes-hits-fixable`

**Scenario:** Given `--replace`, when necro grep runs, then each hit's label says what the match becomes, with `$1` standing for a group.

**Spec:** [Finders](../spec/v0.md#necro-grep)

**Proved by:** `fix/replace.case`

### `write-rewrites-matches`

**Scenario:** Given `--replace` and `--write`, when necro grep runs, then it rewrites the matches and prints the hits it replaced; `--write` or `--only-at` without `--replace` is a usage error.

**Spec:** [Finders](../spec/v0.md#necro-grep)

**Proved by:** `fix/write.case`, `fix/write-needs-replace.case`

### `fix-write-baseline-hints-at-fixable`

**Scenario:** Given `--write-baseline`, when necro writes entries a fix could clear, then it says how many can be fixed automatically and how to keep them out of the baseline.

**Spec:** [Fixes](../spec/v0.md#fixes)

**Proved by:** `fix/write-baseline-hint.case`
