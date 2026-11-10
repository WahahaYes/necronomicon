# necronomicon plugin

House rules that teach: pre-commit checks whose hits explain themselves to coding agents. This plugin ships three skills. `necro` helps an agent work with rule hits: reading them, fixing what is fixable, and recording deliberate exceptions. `necro-rules` helps write new rules and keep agent instructions in step. `necro-adopt` walks through setting necro up in a repo, from install to baseline to hooks.

Install with `/plugin marketplace add WahahaYes/necronomicon` and then `/plugin install necronomicon@necronomicon`. The skills guide the agent; the `necro` binary itself still runs every check, installed separately with `uv tool install necronomicon` or `cargo install necronomicon`. Full documentation lives at https://github.com/WahahaYes/necronomicon
