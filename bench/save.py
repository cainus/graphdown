#!/usr/bin/env python3
"""Records one finished run.   BENCH_OUT=dir bench/save.py RUN TOKENS TOOLS MS [flag...] [< answer]

The answer text comes on stdin, or, when stdin is a terminal or empty, from
the run's own .bench/answer.md (found through BENCH_OUT/plan.json). Flags (such as `invalid` or `duplicate`)
are stored as true and keep a run out of grading."""
import json, os, pathlib, sys

run, tokens, tools, ms, *flags = sys.argv[1:]
record = {
    "run": run,
    "tokens": int(tokens),
    "tool_uses": int(tools),
    "duration_ms": int(ms),
    "answer": "" if sys.stdin.isatty() else sys.stdin.read().strip(),
}
base = pathlib.Path(os.environ.get("BENCH_OUT", "."))
if not record["answer"]:
    plan = {p["run"]: p for p in json.loads((base / "plan.json").read_text())}
    f = pathlib.Path(plan[run]["dir"]) / ".bench" / "answer.md"
    record["answer"] = f.read_text().strip() if f.exists() else "(no answer file)"
for flag in flags:
    record[flag] = True
out = base / "results"
out.mkdir(parents=True, exist_ok=True)
(out / f"{run}.json").write_text(json.dumps(record))
print("saved", run)
