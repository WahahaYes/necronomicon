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

## Clippy

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

## Cargo manifest

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

## Rustfmt

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

## Rust docs

### `rust-docs-missing-item-docs`

**Scenario:** Given `.rs` files, when necro rust-docs checks `missing`, then each public item without a `///` doc is a hit, restricted and test items are exempt, and brackets inside strings, comments and character literals don't skew `#[cfg(test)]` depth.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `rust-docs/missing-lexing.case`, `rust-docs/missing.case`

### `rust-docs-module-doc-required`

**Scenario:** Given `.rs` files, when necro rust-docs checks `module`, then a non-empty file whose first line past blanks and inner attributes isn't `//!` is a hit, while an empty file passes.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `rust-docs/module.case`

### `rust-docs-length-limits`

**Scenario:** Given doc runs, when necro rust-docs checks `length`, then a run longer than its `--header` or `--item` limit is a hit at its first line, with `////` not counted as a doc.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `rust-docs/length.case`

### `rust-docs-pathspecs-narrow-files`

**Scenario:** Given pathspecs, when necro rust-docs runs, then it reads only the tracked `.rs` files they match.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `rust-docs/pathspecs.case`

### `rust-docs-rejects-bad-arguments`

**Scenario:** Given no check or a limit that isn't a whole number of lines, when necro rust-docs runs, then it exits 2 with a usage message.

**Spec:** [Finders](../spec/v0.md#finders)

**Proved by:** `rust-docs/usage.case`
