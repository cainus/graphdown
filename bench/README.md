# Benchmark harness

Scripts for the study planned in @docs/benchmark. Nothing in this folder is copied into the repos agents work on.

- `tasks.json`: each task's prompt, answer key and key sections.
- `setup.sh ARM DEST [SOURCE]`: builds one arm's copy of the corpus, from this repo or any other checkout (@docs/benchmark#arms). Arm B also gets a `graphdown` wrapper in `.bench/bin` that logs every call to `.bench/calls.log`.
- `prompt.py TASK ARM DIR [TASKS.json]`: the prompt for one run. Only arm B's differs, by the CLI path and the skill text.
- `make_runs.py OUT REPOS ARMS REPS [TASKS]`: sets up run folders and prompts for a study on other repos, with neutral shuffled names.
- `save.py RUN TOKENS TOOLS MS`: records a finished run's answer and usage.
- `blind_sheet.py OUT`: builds the blind grading sheet, including each change's diff.
- `grade.sh TASK DEST`: builds a finished run and probes it for the change tasks, printing one JSON line. Questions and judgement calls are graded blind instead.

Run directories get neutral names (`r01`, `r02`, ...) so nothing in a path tells an agent which arm it is in; the mapping lives in the run plan outside the repo.

`pilot-2026-11-25/` holds the raw data from the pilot: the run plan, each run's reported answer and usage, the scripted grades and the blind grading sheet. The write-up is @docs/benchmark-pilot.

`corpus/writer-prompt.txt` is the prompt given to the agents that write tasks for a foreign repo; they are told nothing about graphdown. `pilot-2026-11-25b/` holds the second pilot's data, written up in @docs/benchmark-pilot-2.

`pilot-2026-11-25c/` holds the third pilot (hard tasks, with its screening runs under `screen/`), written up in @docs/benchmark-pilot-3.
