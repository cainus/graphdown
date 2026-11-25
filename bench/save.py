#!/usr/bin/env python3
"""Records one finished run.   BENCH_OUT=dir bench/save.py RUN TOKENS TOOLS MS [flag...] < answer

The answer text comes on stdin. Flags (such as `invalid` or `duplicate`)
are stored as true and keep a run out of grading."""
import json, os, pathlib, sys

run, tokens, tools, ms, *flags = sys.argv[1:]
record = {
    "run": run,
    "tokens": int(tokens),
    "tool_uses": int(tools),
    "duration_ms": int(ms),
    "answer": sys.stdin.read().strip(),
}
for flag in flags:
    record[flag] = True
out = pathlib.Path(os.environ.get("BENCH_OUT", ".")) / "results"
out.mkdir(parents=True, exist_ok=True)
(out / f"{run}.json").write_text(json.dumps(record))
print("saved", run)
