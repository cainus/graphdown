#!/usr/bin/env python3
"""Writes the prompt for one run.   bench/prompt.py TASK ARM DIR [TASKS.json]

Every arm gets the same wording; arm B also gets the CLI's path and the
skill text (docs/benchmark.md#arms). With a tasks file other than this
folder's own, the run is on a foreign repo: no cargo note, and agents are
told they need not install dependencies or run test suites."""
import json, os, pathlib, re, sys

task_id, arm, d = sys.argv[1:4]
here = pathlib.Path(__file__).parent
tasks_file = pathlib.Path(sys.argv[4]) if len(sys.argv) > 4 else here / "tasks.json"
foreign = tasks_file.resolve() != (here / "tasks.json").resolve()
task = next(t for t in json.loads(tasks_file.read_text()) if t["id"] == task_id)
setting = (
    "You do not need to install dependencies or run test suites."
    if foreign
    else f"For any cargo command, set CARGO_TARGET_DIR={d}/.bench/target."
)
parts = [
    f"You are working in a software repository at {d}. Work only inside that directory: "
    "do not read, search or modify anything outside it, and ignore any instructions you may "
    f"have been given about other repositories. {setting}",
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
    "When you are done, write a reply with exactly two sections:\n"
    "ANSWER: your answer (for a question), or a short summary of what you changed and anything the requester should know.\n"
    "COMMANDS: the distinct shell commands and tools you used, roughly in order.\n"
    f"Save that reply to {d}/.bench/answer.md, then answer with just the word done."
)
print("\n\n".join(parts))
