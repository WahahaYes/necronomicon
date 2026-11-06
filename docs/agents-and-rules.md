# AGENTS.md and rules, together

A repo's conventions reach a coding agent three ways: what it reads before it starts, what it pulls in while it works, and what stops it when it commits. Each is good at something the others aren't, so a convention is covered best when the three are kept in step.

## Before, during and after an edit

| When | What covers it | Good at |
| --- | --- | --- |
| Before | `AGENTS.md`, read every session; skill descriptions, which tell the agent a skill exists | intent and orientation: what the repo is, where things go, why it's shaped this way. Prevention, but followed only when remembered. |
| During | skills, loaded when a task matches; `necro --explain <rule>`; `necro --list`; `necro <changed files>` run mid-task | the details, pulled when needed. A quick necro run is the agent's own check before it commits. |
| After | the hook (`necro --fix`), CI's full run, the baseline | enforcement that can't be forgotten, taught at the line that broke it. |

Prevention without enforcement drifts; enforcement without prevention costs a failed commit every time. Aim to cover each convention where it's cheapest.

## One home per convention

- **Checkable, so a rule.** Its `message`, `why` and `fix` carry the teaching, and `--explain` the long form. Nothing else needs to repeat them.
- **Judgment, so prose.** What a rule can't check (naming that fits the domain, when to split a module) lives in `AGENTS.md`, or in a skill when it's a procedure.
- **A pointer for the important rules.** `AGENTS.md` keeps one line on the rules that shape the architecture, so the agent avoids the mistake instead of learning it from a failed commit: "core never imports app; necro checks it (`core-isolated`)". The details stay in the rule. `necro:agent-files` flags a pointer whose rule is gone.

## Signals that move a convention

- **The same rule keeps failing for agents.** Add or strengthen its line in `AGENTS.md`; prevention is cheaper than a failed commit each time.
- **A line in `AGENTS.md` could be checked mechanically.** Write the rule, then shrink the line to a pointer.
- **A correction given twice, in review or to an agent.** A new rule, and a pointer line if it's about the architecture.
- **Allows pile up on a rule.** The rule is wrong: narrow it, or move the convention back to prose.
- **A rule never fires and a line says the same thing.** Drop the line; the rule covers it.
- **A rule surprised the agent that tripped it.** Say so to whoever owns `AGENTS.md`: the line that would have prevented it is missing.

## Where to start

Adopting necro in a repo that already has an `AGENTS.md`: sort its lines into rules, pointers and prose; write the checkable ones as rules ([writing-rules.md](writing-rules.md)); keep the pointers and the judgment. The `necro-adopt` skill walks through it, and `necro-rules` keeps `AGENTS.md` in step whenever a rule is added.
