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
