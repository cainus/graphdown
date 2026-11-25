#!/usr/bin/env python3
"""Builds a blind grading sheet from finished runs.

    bench/blind_sheet.py OUT

Reads OUT/plan.json, OUT/tasks-*.json and OUT/results/<run>.json (answer
text). Writes OUT/blind.txt (shuffled ids, task, key, summary and, for
changes, the run's diff) and OUT/blind-key.json mapping ids to runs.
Runs marked duplicate or invalid are left out."""
import json, pathlib, random, subprocess, sys

out = pathlib.Path(sys.argv[1])
plan = {p["run"]: p for p in json.loads((out / "plan.json").read_text())}
tasks = {}
for f in out.glob("tasks-*.json"):
    for t in json.loads(f.read_text()):
        tasks[(f.stem[6:], t["id"])] = t
res = {}
for f in (out / "results").glob("*.json"):
    r = json.loads(f.read_text())
    if not r.get("duplicate") and not r.get("invalid"):
        res[r["run"]] = r
items = sorted(r for r in res if r in plan)
random.seed(len(items) * 7)
random.shuffle(items)
lines = [
    "You are grading coding-agent runs blind. For each ITEM you get the task, the grading key, the agent's final summary, and for change tasks the actual diff. Judge the work, not the writing.",
    'Output one JSON object per line and nothing else: {"id": "gNN", "score": 0|0.5|1, "hidden_point": true|false|null, "note": "<=20 words"}.',
    "score 1 = fully correct per the key, every point in it; 0.5 = mostly right with a real omission or error; 0 = wrong or missing the main point. For questions judge the answer's facts; for changes judge the diff.",
    "hidden_point: true if the run got the key's hardest or hidden point (the one a hasty agent would miss), false if not.",
    "",
]
key = {}
for i, rid in enumerate(items, 1):
    gid = f"g{i:02d}"
    key[gid] = rid
    p = plan[rid]
    t = tasks[(p["repo"], p["task"])]
    lines += [f"=== ITEM {gid} ({p['repo']} {p['task']}, {t['type']})", "TASK: " + t["prompt"], "KEY: " + (t["key"] if isinstance(t["key"], str) else json.dumps(t["key"], indent=1)),
              "SUMMARY: " + res[rid]["answer"]]
    if t["type"] != "question":
        d = p["dir"]
        diff = subprocess.run(["git", "-C", d, "diff", "HEAD"], capture_output=True, text=True).stdout
        new = subprocess.run(["git", "-C", d, "ls-files", "--others", "--exclude-standard"], capture_output=True, text=True).stdout.split()
        for n in new:
            if not n.startswith(".bench"):
                diff += f"\n+++ new file {n}\n" + (pathlib.Path(d) / n).read_text(errors="replace")[:3000]
        dl = diff.splitlines()
        lines.append("DIFF:\n" + "\n".join(dl[:260]) + ("\n[diff truncated]" if len(dl) > 260 else ""))
    lines.append("")
(out / "blind.txt").write_text("\n".join(lines))
(out / "blind-key.json").write_text(json.dumps(key))
print(len(items), "items")
