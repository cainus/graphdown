# Benchmark harness

Scripts for the study planned in @docs/benchmark. Nothing in this folder is copied into the repos agents work on.

- `tasks.json`: each task's prompt, answer key and key sections.
- `setup.sh ARM DEST`: builds one arm's copy of the corpus (@docs/benchmark#arms). Arm B also gets a `graphdown` wrapper in `.bench/bin` that logs every call to `.bench/calls.log`.
- `prompt.py TASK ARM DIR`: the prompt for one run. Only arm B's differs, by the CLI path and the skill text.
- `grade.sh TASK DEST`: builds a finished run and probes it for the change tasks, printing one JSON line. Questions and judgement calls are graded blind instead.

Run directories get neutral names (`r01`, `r02`, ...) so nothing in a path tells an agent which arm it is in; the mapping lives in the run plan outside the repo.
