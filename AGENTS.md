# necronomicon

`necro` checks a repo against its house rules (`necro.yaml`) and words each hit so the agent or person who tripped it learns the rule. [spec/v0.md](spec/v0.md) is the contract; [tests/conformance/](tests/conformance/) makes it executable. The `necro-dev` skill walks through changing the engine, finders and packs.

## Layout

- `src/main.rs` the CLI; `src/config.rs` loads and validates `necro.yaml`; `src/run.rs` runs rules, then allows, the baseline and fixes; `src/render/` prints.
- `src/finders/` the commands necro bundles (`necro grep`, `necro clippy`, …); `src/packs/` the built-in packs, one YAML file each.
- `.agents/skills/` the skills: `necro` and `necro-rules` are embedded in the binary (`src/skills.rs`, `necro skills install`), `necro-adopt` ships through `npx skills add`, and `necro-dev` (internal) is for this repo.
- `necro.yaml` this repo's own house rules; its `exclude` leaves out the conformance cases and examples, which quote rules and allows as data.
- `spec/v0.md` the format and behavior; `tests/conformance/cases/<area>/*.case` one case per behavior; `examples/` small adopting repos, checked by CI; `docs/writing-rules.md` the adopter guide, and `docs/agents-and-rules.md` how rules and `AGENTS.md` cover a repo together.

## Build and test

Needs the Rust toolchain in `rust-toolchain.toml` and `uv` (the suite and scripted rules run through `uv run --script`).

- `make test`: unit tests and the conformance suite, quiet; failures print in full.
- `make lint`: clippy, then this repo's rules with the current build.
- `make format`, `make check` (every hook in `.pre-commit-config.yaml` on every file, run with prek, as CI runs them).

## Changing behavior

1. Change the spec first, then write or change the cases, then the engine. A case is the spec's executable form; a behavior without one isn't done.
2. Every user-visible change gets a line under `## Unreleased` in `CHANGELOG.md`, and the README or `docs/writing-rules.md` where adopters would look.
3. Rules here explain themselves: a failing commit names the rule, and `necro --explain <rule>` gives the rest. Fix the code, or add an allow comment with a reason.
