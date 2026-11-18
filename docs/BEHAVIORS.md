# Behaviors

What necro's users rely on, one entry per behavior: a slug, a scenario, the spec section it comes from, and the conformance cases that prove it. Each case in `tests/conformance/cases/` names one entry in its `behavior:` header, and the suite fails when a case names a slug that isn't here or an entry's **Proved by** list differs from the cases naming it.

A bug fix adds a case to the behavior it broke. If no entry fits, add the entry first.

## A rule

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

## Packs

### `pack-include`

**Scenario:** Given `include: [necro:<pack>]`, when necro runs, then the pack's rules are added as `<pack>/<rule>`; an unknown pack is a config error.

**Spec:** [Packs](../spec/v0.md#packs)

**Proved by:** `packs/include.case`, `packs/unknown.case`

### `pack-baseline`

**Scenario:** Given a pack rule's hits, when `--write-baseline` runs, then the baseline entries use the rule's full name `<pack>/<rule>`.

**Spec:** [Packs](../spec/v0.md#packs)

**Proved by:** `packs/baseline.case`

### `pack-alongside-repo-rules`

**Scenario:** Given an included pack and the repo's own rules, when necro runs, then pack rules sit beside repo rules and globs, and `--rule` takes a pack rule's full name.

**Spec:** [Packs](../spec/v0.md#packs)

**Proved by:** `packs/beside-repo-rules.case`

### `pack-disable`

**Scenario:** Given `disable: [<pack>/<rule>…]`, when necro runs, then those pack rules are turned off; a name that isn't a rule of an included pack is a config error; an allow for a disabled rule suppresses nothing and becomes unused.

**Spec:** [Packs](../spec/v0.md#packs)

**Proved by:** `packs/disable.case`, `packs/disable-unknown.case`, `packs/disabled-allow.case`

### `pack-override`

**Scenario:** Given `override: {<pack>/<rule>: {files, exclude, params}}`, when necro runs, then the pack rule's `files`, `exclude` and parameters are replaced; a name not in an included pack or one `disable` turns off is a config error; an overridden parameter must be declared by the rule with a value of the same kind.

**Spec:** [Packs](../spec/v0.md#packs)

**Proved by:** `packs/override.case`, `packs/override-files.case`, `packs/override-params.case`, `packs/override-bad.case`, `packs/override-params-bad.case`

### `pack-explain`

**Scenario:** Given `--explain <pack>/<rule>`, when necro runs, then it shows the rule's message, why, fix, details, examples, command with files, exclude and parameters marked `(override)`, and the pack it comes from.

**Spec:** [Packs](../spec/v0.md#packs)

**Proved by:** `packs/explain.case`, `packs/explain-override.case`

### `pack-allow`

**Scenario:** Given an allow comment naming a pack rule by its full name `<pack>/<rule>`, when necro runs, then the allow suppresses the hit.

**Spec:** [Packs](../spec/v0.md#packs)

**Proved by:** `packs/allow.case`

### `rust-style-pack`

**Scenario:** Given `include: [necro:rust-style]`, when necro runs, then it enforces a doc on every public item, a `//!` on every file, doc blocks within line limits, no decorative comment borders, and no role-word type names; parameters tune the limits.

**Spec:** [Packs](../spec/v0.md#necorust-style)

**Proved by:** `packs/rust-style.case`

### `rust-clippy-pack`

**Scenario:** Given `include: [necro:rust]`, when necro runs, then `rust/clippy` reports compiler and clippy warnings labelled with their lint, and `override` with `skip` drops lints by name.

**Spec:** [Packs](../spec/v0.md#necorust)

**Proved by:** `packs/rust-clippy.case`

### `gdscript-pack`

**Scenario:** Given `include: [necro:gdscript]`, when necro runs, then it runs gdtoolkit's formatter, linter and parser, and its own guards for inline lambdas, tracked `.uid` files, Godot 3 legacy API, physics context, literal loads, class-name preloads, signal-name prefixes, and legacy TileMap.

**Spec:** [Packs](../spec/v0.md#necrogdscript)

**Proved by:** `packs/gdscript.case`, `packs/gdscript-class-by-name.case`, `packs/gdscript-legacy-3x.case`, `packs/gdscript-literal-load.case`, `packs/gdscript-physics-context.case`, `packs/gdscript-signal-name.case`, `packs/gdscript-tilemap-legacy.case`

### `gdscript-style-pack`

**Scenario:** Given `include: [necro:gdscript-style]`, when necro runs, then it flags inferred types with `:=`, `$` paths deeper than two nodes, `get_node()` through `..`, and signals connected straight to another emit.

**Spec:** [Packs](../spec/v0.md#necrogdscript-style)

**Proved by:** `packs/gdscript-style.case`

### `gdscript-fix`

**Scenario:** Given `include: [necro:gdscript]` and `--fix`, when necro runs, then it runs `gdformat` through the pack and reports remaining hits.

**Spec:** [Fixes](../spec/v0.md#fixes)

**Proved by:** `packs/gdscript-fix.case`

### `godot-arch-pack`

**Scenario:** Given `include: [necro:godot-arch]`, when necro runs, then it flags comments in Godot data files, bare `assert()` in tests, `ResourceSaver` in game code outside `addons/`, children reaching up through `get_parent()` or `owner`, and node behavior in `Resource` scripts.

**Spec:** [Packs](../spec/v0.md#necrogodot-arch)

**Proved by:** `packs/godot-arch-data-comment.case`, `packs/godot-arch-no-bare-assert.case`, `packs/godot-arch-no-res-writes.case`, `packs/godot-arch-parent-call.case`, `packs/godot-arch-resource-behavior.case`

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

## `necro grep`

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

## `necro clippy`

### `clippy-checks-each-workspace`

**Scenario:** Given a working folder with Cargo workspaces below it or enclosing it, when necro clippy runs, then it checks every one, keeps only hits under the working folder, and finds nothing when there is no workspace.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `clippy/enclosing.case`, `clippy/no-workspace.case`, `clippy/workspaces.case`

### `clippy-reports-primary-spans`

**Scenario:** Given a lint hit, when necro clippy runs, then each hit is the diagnostic's primary span with clippy's message, and two hits on one line are one at the leftmost column.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `clippy/one-per-line.case`, `clippy/unwrap.case`

### `clippy-tests-includes-test-code`

**Scenario:** Given test targets and `#[cfg(test)]` code, when necro clippy runs with `--tests`, then it reports hits in them too.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `clippy/tests.case`

### `clippy-match-skip-filter-hits`

**Scenario:** Given hits, when necro clippy runs with `--match` and `--skip`, then it keeps hits whose span text matches the first regex and drops those matching the second.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `clippy/match-skip.case`

### `clippy-others-reports-other-warnings`

**Scenario:** Given a workspace, when necro clippy runs with `--others`, then it reports every other warning labelled with its lint, leaving out necro's own lints, and `--except` drops lints by name with or without `clippy::`.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `clippy/others-except.case`, `clippy/others.case`

### `clippy-rejects-bad-lint-arguments`

**Scenario:** Given a missing, unknown or out-of-place lint argument, when necro clippy runs, then it exits 2 with a usage message.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `clippy/no-lint.case`, `clippy/others-usage.case`, `clippy/unknown-lint.case`

## `necro cargo-manifest`

### `cargo-manifest-checks-edition`

**Scenario:** Given Cargo members, when necro cargo-manifest runs, then each member older than the target edition (2024, or `--edition`) is a hit at its edition line, or the whole manifest when it has none, and a member on the target is clean.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `cargo-manifest/clean.case`, `cargo-manifest/edition.case`, `cargo-manifest/no-edition.case`

### `cargo-manifest-requires-rust-version`

**Scenario:** Given a publishable member with no `rust-version`, when necro cargo-manifest runs, then it is a whole-file hit.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `cargo-manifest/old.case`

### `cargo-manifest-resolves-workspace-inheritance`

**Scenario:** Given members that inherit `edition` and `rust-version` through a workspace, when necro cargo-manifest runs, then it resolves them, hits each member's own edition line, and checks every workspace.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `cargo-manifest/workspace.case`

### `cargo-manifest-rejects-arguments`

**Scenario:** Given an argument, when necro cargo-manifest runs, then it exits 2 with a usage message.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `cargo-manifest/usage.case`

## `necro rustfmt`

### `rustfmt-reports-unformatted-files`

**Scenario:** Given a Cargo workspace, when necro rustfmt runs, then it prints a whole-file hit for each file under the working folder that `cargo fmt --check` would change.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `rustfmt/check.case`

### `rustfmt-fix-reformats-hit-files`

**Scenario:** Given unformatted files and an allowed one, when necro runs with `--fix`, then rustfmt reformats the hit files and leaves the allowed one alone.

**Spec:** [Finders](../spec/v0.md#finders), [Fixes](../spec/v0.md#fixes)

**Proved by:** `rustfmt/fix.case`

### `rustfmt-rejects-unknown-options`

**Scenario:** Given an unknown option, when necro rustfmt runs, then it exits 2 with a usage message naming `--write` and `--only-at`.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `rustfmt/usage.case`

## `necro rust-docs`

### `rust-docs-missing-item-docs`

**Scenario:** Given `.rs` files, when necro rust-docs checks `missing`, then each public item without a `///` doc is a hit, restricted and test items are exempt, and brackets inside strings, comments and character literals don't skew `#[cfg(test)]` depth.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `rust-docs/missing-lexing.case`, `rust-docs/missing.case`

### `rust-docs-module-doc-required`

**Scenario:** Given `.rs` files, when necro rust-docs checks `module`, then a non-empty file whose first line past blanks, plain `//` comments (a license header) and inner attributes isn't `//!` is a hit, while an empty file passes.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `rust-docs/module.case`

### `rust-docs-length-limits`

**Scenario:** Given doc runs, when necro rust-docs checks `length`, then a run longer than its `--header` or `--item` limit is a hit at its first line, with `////` not counted as a doc.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `rust-docs/length.case`

### `rust-docs-pathspecs-narrow-files`

**Scenario:** Given pathspecs, when necro rust-docs runs, then it reads only the `.rs` files they match.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `rust-docs/pathspecs.case`

### `rust-docs-rejects-bad-arguments`

**Scenario:** Given no check or a limit that isn't a whole number of lines, when necro rust-docs runs, then it exits 2 with a usage message.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `rust-docs/usage.case`

## `necro gdscan`

### `class-by-name-finds-scripts-used-by-path`

**Scenario:** Given a script that declares a `class_name`, when another script uses its `res://` path in `preload`, `load`, `extends` or a const, then `class-by-name` reports each such path, relative to the folder holding `project.godot` and including paths outside a narrowed run's files.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `gdscan/class-by-name.case`, `gdscan/class-by-name-project.case`

### `godot-data-files-hold-no-comments`

**Scenario:** Given a full-line `;` comment in a Godot data file, when `data-comment` runs, then it reports the comment's line, but a `;` inside a multi-line string value and the header Godot writes atop `project.godot` are not comments.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `gdscan/data-comment.case`

### `doc-length-limits-header-and-item-runs`

**Scenario:** Given own-line `##` runs, when `doc-length` runs, then the first run, when only the preamble (blank lines, `#!`, plain `#` comments, `@tool`, `@icon`, `class_name`, `extends`) comes before it, gets `--header` while any other gets `--item`, each over-limit run reported at its first line.

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

### `res-write-flags-resourcesaver`

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

## `necro gdtoolkit`

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

## `necro header`

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

## `necro agent-files`

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

## Allows

### `allow-suppresses-hit`

**Scenario:** Given a hit, when the file has `necro-allow <rule>: <reason>` in a comment on the hit's line or the line above (line 1, or 2 after `#!`, for a whole-file hit), then the hit is suppressed; one comment may carry allows for two rules.

**Spec:** [Allows](../spec/v0.md#allows)

**Proved by:** `allows/suppress.case`, `allows/two-on-a-line.case`, `allows/untracked.case`, `allows/whole-file.case`

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
