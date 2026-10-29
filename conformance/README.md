# Conformance suite

Executable cases for [spec/v0.md](../spec/v0.md), run against any engine command:

```
python3 conformance/run.py [-k TEXT] [-v] -- <engine argv>
```

`cargo test` runs it against the built `necro`. Each case copies `base/` into a fresh temp dir, lays its own files over it, makes it a git repo with everything added, and runs the engine there with `NO_COLOR=1`, `CI` unset and the engine on `PATH` as `necro`.

## Case format

One file per case, `cases/<area>/<name>.case`, read top to bottom:

```
# what the case shows
spec: <section>
args: --format terse        # shell-split
exit: 1                     # default 0
env: KEY=value …            # optional
git: false                  # optional: leave it outside git
-- src/a.ts --              # a file laid over base/; an empty body deletes it
…
-- @stdout --               # expected stdout; absent means empty
-- @stderr --               # expected stderr; absent means empty
-- @config-error --         # instead of the two above: one `file:line:col: key.path` per line
-- @file necro.baseline --  # optional: a file's contents after the run
```

Comparison drops `\r`, trailing whitespace and trailing blank lines, and writes the temp root as `<root>`; nothing else is normalised. `@config-error` expects exit 2, empty stdout, and stderr lines `necro: config error: <file:line:col: key.path>: <any message>`, in order.

## Base

`necro.yaml` has one rule, `banned-word`: `git grep` for the word `donkey` under `src/`. `necro/t.py` is a canned command for parsing cases: run as `uv run --script "$NECRO_ROOT_DIR/necro/t.py"`, it prints `canned.txt` and `canned-stderr.txt` from its working folder and exits with `T_EXIT`.
