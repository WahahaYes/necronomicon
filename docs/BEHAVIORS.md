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
