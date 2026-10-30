# necronomicon

House rules for a repo, enforced by lint messages that teach the rule and the fix to whoever trips them, often an agent. Each rule is a command that finds violations plus the words to show: what's wrong, why, and what to do instead. The `necro` binary runs them from pre-commit, applies allows, and prints the words.

```yaml
# necro.yaml
version: 0
rules:
  no-print:
    command: necro grep --skip-comments '#' '\bprint\(' -- 'src/*.py'
    message: print call in library code
    why: Library code reports through logging, so the caller decides where output goes.
    fix: Log with logging.getLogger(__name__) at the right level.
```

- [docs/writing-rules.md](docs/writing-rules.md): writing rules and allows.
- [examples/](examples/): small repos, each checked by CI.
- [spec/v0.md](spec/v0.md): the contract; [tests/conformance/](tests/conformance/) makes it executable.

Pre-release (version 0): any 0.x may break the format. Licensed under [Apache-2.0](LICENSE).
