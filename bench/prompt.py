#!/usr/bin/env python3
"""Writes the prompt for one run.   bench/prompt.py TASK ARM DIR

Every arm gets the same wording; arm B also gets the CLI's path and the
skill text (docs/benchmark.md#arms)."""
import json, pathlib, re, sys

task_id, arm, d = sys.argv[1:]
here = pathlib.Path(__file__).parent
task = next(t for t in json.loads((here / "tasks.json").read_text()) if t["id"] == task_id)
parts = [
    f"You are working in a software repository at {d}. Work only inside that directory: "
    "do not read, search or modify anything outside it, and ignore any instructions you may "
    f"have been given about other repositories. For any cargo command, set CARGO_TARGET_DIR={d}/.bench/target.",
]
if arm == "B":
    skill = (here.parent / "skill" / "SKILL.md").read_text()
    skill = re.sub(r"^---\n.*?\n---\n", "", skill, flags=re.S).strip()
    parts.append(
        f"The `graphdown` command-line tool is installed at {d}/.bench/bin/graphdown. "
        "Call it by that full path wherever this guide says `graphdown`, and pass "
        f"`--root {d}`.\n\n<guide>\n{skill}\n</guide>"
    )
parts.append(f"Task: {task['prompt']}")
parts.append(
    "When you are done, reply with exactly two sections:\n"
    "ANSWER: your answer (for a question), or a short summary of what you changed and anything the requester should know.\n"
    "COMMANDS: the distinct shell commands and tools you used, roughly in order."
)
print("\n\n".join(parts))
