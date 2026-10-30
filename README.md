# necronomicon

House rules for a repo, enforced by lint messages that teach the rule and the fix to whoever trips them, often an agent. Each rule is a command that finds violations plus the words to show: what's wrong, why, and what to do instead. The `necro` binary runs them from pre-commit, applies allows, and prints the words.

![You write a rule; an agent's commit breaks it; pre-commit blocks it and pushes the why; the agent pulls the rest and commits the fix](docs/mind-control.svg)

## Agent mind control

A rule costs an agent nothing until it breaks one. Then necro tells it just enough, at the line it broke, and the rest is one command away. From [examples/layers](examples/layers):

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

- [docs/writing-rules.md](docs/writing-rules.md): writing rules and allows.
- [examples/](examples/): small repos, each checked by CI.
- [spec/v0.md](spec/v0.md): the contract; [tests/conformance/](tests/conformance/) makes it executable.

Pre-release (version 0): any 0.x may break the format. Licensed under [Apache-2.0](LICENSE).
