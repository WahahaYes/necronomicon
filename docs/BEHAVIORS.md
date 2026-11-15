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

## Gdscan

### `class-by-name-finds-scripts-used-by-path`

**Scenario:** Given a script that declares a `class_name`, when another script uses its `res://` path in `preload`, `load`, `extends` or a const, then `class-by-name` reports each such path, relative to the folder holding `project.godot` and including paths outside a narrowed run's files.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `gdscan/class-by-name.case`, `gdscan/class-by-name-project.case`

### `godot-data-files-hold-no-comments`

**Scenario:** Given a full-line `;` comment in a Godot data file, when `data-comment` runs, then it reports the comment's line, but a `;` inside a multi-line string value and the header Godot writes atop `project.godot` are not comments.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `gdscan/data-comment.case`

### `doc-length-limits-header-and-item-runs`

**Scenario:** Given own-line `##` runs, when `doc-length` runs, then the run after only plain `#` preamble gets `--header` while any other gets `--item`, each over-limit run reported at its first line.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `gdscan/doc-length.case`, `gdscan/doc-length-after-comment.case`

### `exports-need-a-doc-comment`

**Scenario:** Given an `@export` line, when no own-line `##` comment sits directly above it, then `export-doc` reports it, counting `@export_range` and the like but not a plain `#`.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `gdscan/export-doc.case`

### `inline-lambdas-in-brackets`

**Scenario:** Given a `func(` lambda inside an open bracket, when `inline-lambda` runs, then it reports the lambda whether the bracket opened on that line or an earlier one, while a lambda outside brackets or a bracket inside a triple-quoted string is fine.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `gdscan/inline-lambda.case`, `gdscan/inline-lambda-string-bracket.case`

### `physics-calls-only-in-physics-process`

**Scenario:** Given a physics-step call inside `func _process`, when `physics-context` runs, then it reports the call, while the same call in `_physics_process`, a longer identifier, a string or a comment is fine.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `gdscan/physics-context.case`

### `resource-saver-only-in-code`

**Scenario:** Given `ResourceSaver` in code, when `res-write` runs, then it reports it, but not in a top-level `@tool` script, a string or a comment.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `gdscan/res-write.case`

### `top-level-funcs-need-two-blank-lines`

**Scenario:** Given a top-level `func` or `static func` with the comments attached above it, when `spacing` runs, then it reports fewer than two blank lines before it, except the first, and a bracket inside a triple-quoted string doesn't change that.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `gdscan/spacing.case`, `gdscan/spacing-string-bracket.case`

### `spacing-fix-adds-the-blank-lines`

**Scenario:** Given `spacing` hits, when `necro --fix` runs, then it puts two blank lines above each block, attached comments included, and joins a top-level comment that blank lines keep from its function.

**Spec:** [Finders](../spec/v0.md#finders), [Fixes](../spec/v0.md#fixes)

**Proved by:** `gdscan/spacing-fix.case`

### `uid-files-are-tracked-together`

**Scenario:** Given a repo that tracks `.gd.uid` files, when `uid` runs, then each tracked `.gd` needs a tracked `.gd.uid` and each tracked `.gd.uid` its `.gd`, while a repo tracking none or an untracked script finds nothing.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `gdscan/uid.case`, `gdscan/uid-none.case`, `gdscan/uid-untracked.case`

### `gdscan-rejects-an-unknown-check`

**Scenario:** Given a check `gdscan` doesn't know, when `gdscan` runs, then it exits 2 with the usage line naming the checks it takes.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `gdscan/usage.case`

### `only-spacing-can-write`

**Scenario:** Given `--write` with a check other than `spacing`, when `gdscan` runs, then it exits 2 saying `--write` goes with `spacing`.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `gdscan/write-usage.case`

## Gdtoolkit

### `format-flags-files-gdformat-would-change`

**Scenario:** Given a `.gd` file `gdformat` would change, when `gdtoolkit format` runs, then it is a whole-file hit, and one holding a lambda inside brackets says `--fix` skips it.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `gdtoolkit/format.case`

### `lint-reports-problems-at-their-lines`

**Scenario:** Given `gdlint` problems, when `gdtoolkit lint` runs, then each is at its line with the check in parentheses, `max-returns` at the function's `func` line, and hits sort by line number.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `gdtoolkit/lint.case`

### `missing-gdtoolkit-names-the-extra`

**Scenario:** Given no `gdlint` on `PATH`, when `gdtoolkit lint` runs, then it exits 2 naming the extra that installs gdtoolkit beside necro.

**Spec:** [Finders](../spec/v0.md#finders), [Packs](../spec/v0.md#packs)

**Proved by:** `gdtoolkit/missing.case`

### `parse-reports-unparseable-files`

**Scenario:** Given a file `gdtoolkit` can't parse, when `gdtoolkit parse` runs, then it reports the position `gdtoolkit` gives, leaving that file to this check rather than lint or format.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `gdtoolkit/parse.case`

### `write-reformats-files`

**Scenario:** Given files `gdformat` would change, when `gdtoolkit --write format` runs, then it reformats them except one with a lambda it corrupts, naming each file it reformatted.

**Spec:** [Finders](../spec/v0.md#finders), [Fixes](../spec/v0.md#fixes)

**Proved by:** `gdtoolkit/write.case`
