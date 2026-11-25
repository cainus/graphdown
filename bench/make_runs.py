#!/usr/bin/env python3
"""Creates run folders and prompts for a study on foreign repos.

    bench/make_runs.py OUT REPOS ARMS REPS [TASK_IDS]

OUT     folder for runs/, prompts/ and plan.json
REPOS   comma list of name=checkout, with tasks in OUT/tasks-<name>.json
ARMS    e.g. AB or A
REPS    runs per task per arm
TASK_IDS optional comma list of repo:task to include (default: all)

Run folders get neutral, shuffled names (docs/benchmark.md#harness)."""
import json, os, pathlib, random, subprocess, sys

here = pathlib.Path(__file__).parent
out, repos, arms, reps = sys.argv[1], sys.argv[2], sys.argv[3], int(sys.argv[4])
only = set(sys.argv[5].split(",")) if len(sys.argv) > 5 else None
out = pathlib.Path(out)
(out / "runs").mkdir(parents=True, exist_ok=True)
(out / "prompts").mkdir(exist_ok=True)
checkouts = dict(r.split("=", 1) for r in repos.split(","))
runs = []
for name in checkouts:
    for t in json.loads((out / f"tasks-{name}.json").read_text()):
        if only and f"{name}:{t['id']}" not in only:
            continue
        runs += [(name, t["id"], a, r) for a in arms for r in range(1, reps + 1)]
random.seed(len(runs))
random.shuffle(runs)
prefix = out.name[-2:]
plan = []
env = {"GRAPHDOWN_REAL": os.environ.get("GRAPHDOWN_REAL", "/tmp/graphdown-real"), "PATH": "/usr/bin:/bin:/usr/local/bin"}
for i, (name, tid, arm, rep) in enumerate(runs, 1):
    rid = f"{prefix}{i:02d}"
    d = out / "runs" / rid
    subprocess.run([str(here / "setup.sh"), arm, str(d), checkouts[name]], check=True, stdout=subprocess.DEVNULL, env=env)
    prompt = subprocess.run([str(here / "prompt.py"), tid, arm, str(d), str(out / f"tasks-{name}.json")],
                            capture_output=True, text=True, check=True).stdout
    (out / "prompts" / f"{rid}.txt").write_text(prompt)
    plan.append({"run": rid, "repo": name, "task": tid, "arm": arm, "rep": rep, "dir": str(d)})
(out / "plan.json").write_text(json.dumps(plan, indent=1))
print(" ".join(f"{p['run']}:{p['repo']}/{p['task']}/{p['arm']}" for p in plan))
