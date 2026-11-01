<p align="center"><img src="https://raw.githubusercontent.com/WahahaYes/necronomicon/main/docs/banner.png" width="640" alt="necronomicon: lint errors that explain themselves to coding agents. A spiral-eyed pixel robot, wrapped by a green tentacle, takes a dashed feed from a glowing, floating book."></p>

# necronomicon

A simple way to organize a repo's pre-commit hooks for progressive disclosure, steering coding agents toward the repo's conventions and intended architecture. Each rule pairs a command that finds violations with what to say about them: what's wrong, why the rule exists, and what to do instead. An agent sees a rule only when its commit breaks it, at the line it broke, with the long form one `necro --explain` away.

![You write a rule; an agent's commit breaks it; pre-commit blocks it and pushes the why; the agent pulls the rest and commits the fix](https://raw.githubusercontent.com/WahahaYes/necronomicon/main/docs/mind-control.svg)

## Why not AGENTS.md

An instruction file is read every session, whether or not today's work touches it, and followed when the agent remembers to. A rule costs nothing until it's broken, and then it blocks the commit. Keep AGENTS.md for orientation and move its "never do X" lines into rules.

## Agent mind control

When an agent trips a rule, necro tells it just enough, and the rest is one command away. From [examples/layers](https://github.com/WahahaYes/necronomicon/tree/main/examples/layers):

```yaml
core-isolated:
  command: necro grep '^\s*(from|import)\s+app\b' -- 'src/core/*.py'
  message: core imports the app layer
  why: Core runs in tests without a database; importing app/ drags one in.
  fix: Take a port from core/ports.py as an argument; app/ passes the real one in.
  details: …   # the long form, and bad and good examples, for necro --explain
```

<details>
<summary><b>An agent that never read the rule commits <code>from app.db import session</code> in core.</b> The commit fails, and 11 lines land in its context.</summary>

```text
error[core-isolated]: core imports the app layer
 --> src/core/pricing.py:2:1
  |
2 | from app.db import session
  | ^^^^^^^^
  |
  = why: Core runs in tests without a database; importing app/ drags one in.
  = fix: Take a port from core/ports.py as an argument; app/ passes the real one in.
  = more: necro --explain core-isolated

necro: 1 error (core-isolated ×1)
```

<details>
<summary><b>It wants the bigger picture</b>, so it runs the command it was handed and pulls 22 more.</summary>

```text
$ necro --explain core-isolated
core-isolated: core imports the app layer

Why: Core runs in tests without a database; importing app/ drags one in.

Fix: Take a port from core/ports.py as an argument; app/ passes the real one in.

Core is the inner layer: app/ may import core, never the reverse.
Exceptions: none. Shared types move into core.

Bad:
    from app.db import session

    def price(order):
        return session.rates(order.region) * order.total

Good:
    from core.ports import Rates

    def price(order, rates: Rates):
        return rates.for_region(order.region) * order.total

Command: necro grep '^\s*(from|import)\s+app\b' -- 'src/core/*.py'
```

</details>
</details>

**Re-steered,** it takes `rates: Rates` as an argument, and the commit passes.

## What necro is, and isn't

- **An organizer, not a parser.** A rule's `command` is anything that prints one `path[:line[:col]]` per violation: `necro grep` for a pattern, a short script for more, or a tool you already run. necro owns the wording, allows and output, so rules work in any language.
- **A replacement for the lint script you'd write anyway.** Repos that work with agents tend to grow a `lint_style.py` that greps for house rules and prints one-line errors. necro keeps those rules as config and adds the why, the fix and `--explain`.
- **Exceptions with reasons.** `necro-allow <rule>: <reason>` in a comment on the line or the one above. An allow without a reason, or one that suppresses nothing, fails the run.
- **A ratchet for rollout.** Grandfather existing hits in a committed snapshot so new ones fail alone; refresh it deliberately as debt gets fixed. See [Baseline](https://github.com/WahahaYes/necronomicon/blob/main/spec/v0.md#baseline).

## Install

```sh
uv tool install necronomicon   # PyPI wheel around the necro binary, no Rust needed
pip install necronomicon       # the same wheel, into the current environment
cargo install necronomicon     # from crates.io, built from source
```

As a pre-commit hook, which installs the wheel into pre-commit's own environment:

```yaml
- repo: local
  hooks:
    - id: necro
      name: necro
      entry: necro
      language: python
      additional_dependencies: ['necronomicon']
      pass_filenames: false
      always_run: true
```

Pin below the next 0.x: any 0.x may break the format.

## More

- [docs/writing-rules.md](https://github.com/WahahaYes/necronomicon/blob/main/docs/writing-rules.md): writing rules and allows, and the pre-commit hook.
- [examples/](https://github.com/WahahaYes/necronomicon/tree/main/examples): small repos, each checked by CI.
- [spec/v0.md](https://github.com/WahahaYes/necronomicon/blob/main/spec/v0.md): the contract; [tests/conformance/](https://github.com/WahahaYes/necronomicon/tree/main/tests/conformance) makes it executable.

Pre-release (version 0): any 0.x may break the format. Licensed under [Apache-2.0](https://github.com/WahahaYes/necronomicon/blob/main/LICENSE).
