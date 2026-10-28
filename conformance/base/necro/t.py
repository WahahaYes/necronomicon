"""The suite's scripted checks, speaking the check protocol directly (no helper).

T_MODE makes it misbehave on purpose: exit1, garbage, sleep, proto2, badvalue,
outside, noresult, error.
"""

import json
import os
import sys
import time

MODE = os.environ.get("T_MODE", "")
ROOT = os.environ["NECRO_ROOT"]
CHECKS = {
    "ban": {"scope": "file", "params": {"word": {"type": "string", "default": "bad"}}, "values": ["word"]},
    "all": {"scope": "project", "values": []},
    "calls": {"scope": "file", "params": {"tag": {"type": "string", "default": ""}}, "values": []},
}


def ban(job):
    word, hits = job["params"]["word"], []
    for path in job["files"]:
        with open(os.path.join(ROOT, path), encoding="utf-8") as f:
            for n, line in enumerate(f.read().split("\n"), 1):
                if (c := line.find(word)) >= 0:
                    values = {"word": word, "other": "x"} if MODE == "badvalue" else {"word": word}
                    hits.append({"file": path, "line": n, "column": c + 1, "length": len(word), "values": values})
    if MODE == "outside" and hits:
        hits[0]["file"] = "src/elsewhere.ts"
    return hits


def answer(msg):
    if msg["type"] == "describe":
        return {"protocol": 2 if MODE == "proto2" else 1, "checks": CHECKS}
    if MODE == "exit1":
        print("t.py: something broke", file=sys.stderr)
        sys.exit(1)
    if MODE == "sleep":
        time.sleep(30)
    if MODE == "garbage":
        return "not json"
    results, warnings, errors = {}, [], []
    for job in msg["jobs"]:
        if job["check"] == "ban":
            results[job["id"]] = {"hits": ban(job)}
        elif job["check"] == "all":
            results[job["id"]] = {"hits": []}
            warnings.append({"job": job["id"], "message": f"saw {len(job['files'])} files"})
        else:
            results[job["id"]] = {"hits": []}
            warnings.append({"message": f"called with {json.dumps(job['params'], sort_keys=True)}"})
    if MODE == "noresult":
        results.clear()
    if MODE == "error":
        errors.append({"job": "1", "message": "can't parse src/a.ts"})
    return {"protocol": 1, "results": results, "warnings": warnings, "errors": errors}


for line in sys.stdin:
    reply = answer(json.loads(line))
    print(reply if isinstance(reply, str) else json.dumps(reply), flush=True)
