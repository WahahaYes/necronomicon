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
