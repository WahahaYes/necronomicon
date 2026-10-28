# necronomicon

House rules for a repo, enforced by lint messages that teach the rule and the fix to whoever trips them, often an agent. Rules live in the repo as YAML: what's wrong, why, a valid change and a longer explanation. Checks are regexes, built-in checks, or scripts in the repo's own language. The `necro` binary runs them from pre-commit.

Pre-release: nothing works yet beyond `necro --version`. The spec is [spec/v0.md](spec/v0.md).

Licensed under [Apache-2.0](LICENSE).
