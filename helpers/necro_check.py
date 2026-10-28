"""necro check helper (check protocol 1). Copy this file beside your group's checks.

    from necro_check import check, run

    @check(params={"limit": ("number", 100)}, values=["width"])
    def long_lines(path, text, params):
        for n, line in enumerate(text.split("\\n"), 1):
            if len(line) > params["limit"]:
                yield {"line": n, "column": params["limit"] + 1, "values": {"width": len(line)}}

    run()

A file check yields hits for one file; a project check (scope="project") gets
{path: text} for every file and yields hits with "file". Yield {"warning": …}
to warn. An exception becomes the job's error.
"""

import json
import os
import sys

_checks = {}


def check(scope="file", params=None, values=(), name=None):
    """Declare a check. params: {name: (type, default)}; leave out the default to require it."""

    def register(fn):
        decl = {"scope": scope, "values": list(values), "params": {}}
        for pname, spec in (params or {}).items():
            decl["params"][pname] = {"type": spec[0], **({"default": spec[1]} if len(spec) > 1 else {})}
        _checks[name or fn.__name__] = (decl, fn)
        return fn

    return register


def _job(job, root, reply):
    decl, fn = _checks[job["check"]]
    texts = {}
    for path in job["files"]:
        with open(os.path.join(root, path), encoding="utf-8", errors="replace") as f:
            texts[path] = f.read()
    if decl["scope"] == "project":
        found = fn(texts, job["params"])
    else:
        found = (dict(hit, file=path) for path, text in texts.items() for hit in fn(path, text, job["params"]))
    hits = []
    for hit in found:
        if "warning" in hit:
            reply["warnings"].append({"job": job["id"], "message": hit["warning"]})
        else:
            hits.append(hit)
    return hits


def run():
    """Answer the engine's messages on stdin until it closes."""
    root = os.environ.get("NECRO_ROOT", ".")
    for line in sys.stdin:
        msg = json.loads(line)
        if msg["type"] == "describe":
            reply = {"checks": {n: decl for n, (decl, _) in _checks.items()}}
        else:
            reply = {"results": {}, "warnings": [], "errors": []}
            for job in msg["jobs"]:
                try:
                    reply["results"][job["id"]] = {"hits": _job(job, root, reply)}
                except Exception as e:  # the engine reports it as a check error
                    reply["errors"].append({"job": job["id"], "message": f"{type(e).__name__}: {e}"})
        print(json.dumps({"protocol": 1, **reply}), flush=True)
