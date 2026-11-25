# Benchmark pilot 5: changed code in `check`, 2026-11-25

A targeted rerun of one task from @docs/benchmark-pilot-4. There, every run of the vite flag task missed the "Migration Guide" paragraph of `docs/changes/ssr-using-modulerunner.md`, and no arm-B agent asked `refs` about it. Since then `check` lists the lines that name changed code (@docs/cli#changed-code). This pilot asks one question: does that list reach the agent, and does the agent act on it? Raw data is in `bench/pilot-2026-11-25e/`.

## Setup

- **Task:** vite c2 only (split `ssrFixStacktrace` and `ssrRewriteStacktrace` into a new future flag), with the same grading key as pilots 3 and 4.
- **Runs:** 5 per arm, 10 runs. Arm B's skill now says to open each changed-code line `check` lists.
- **Grading:** two blind graders, who agreed on all 10.

## Results

| Arm | Scores | Tokens above baseline | Seconds |
| --- | ------ | --------------------- | ------- |
| A   | 0.5 ×5 | 10.2k                 | 45      |
| B   | 0.5 ×5 | 16.7k                 | 52      |

Another tie, but the call logs and answers show where things stand now:

- **The list reaches the agent every time.** All 5 arm-B runs ran `check` before finishing, and in each the output listed `docs/changes/ssr-using-modulerunner.md:23 (#migration-guide)`. In pilot 4 no tool output ever showed an agent that line.
- **The agents read it and decided it was fine.** Two runs named it in their answers as a migration note that is still correct and "doesn't mention any flags"; the other three called the flagged lines in general still correct. None edited it. Arm A never saw the line at all.
- **The agents have a case.** The paragraph says the two methods aren't needed with the Module Runner, which is still true. The key wants it to also name `future.removeSsrFixStacktrace`. That's a reasonable editorial call, but it isn't the clear-cut staleness the task was written to test. The graders followed the key, so every run lost half a point for it.

So the navigation problem from pilot 4 is solved: the line that every agent missed is now in front of every graphdown agent. What's left is a judgement about what the paragraph should say, and tooling can't make that for the agent.

## What this suggests

- **Keep the changed-code list.** It turned a 0 of 4 discovery rate into 5 of 5 at no extra calls, since agents already run `check`.
- **Retire vite c2 as a test of graphdown.** Its remaining point is about editorial judgement, not finding docs. The next study needs tasks where the doc the agent must find is plainly wrong after the change, and keys reviewed for that before any runs.
- **Then the larger study** described in @docs/benchmark, with the fixes from pilots 3 to 5 in place.
