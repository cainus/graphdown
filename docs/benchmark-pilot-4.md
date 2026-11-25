# Benchmark pilot 4: mentions and the finishing step, 2026-11-25

A rerun of @docs/benchmark-pilot-3's four hard tasks after acting on its two suggestions: `refs` now lists unlinked mentions (@docs/cli#refs), and the skill tells agents to check mentions and to outline the doc folders a change touches before finishing. Raw data is in `bench/pilot-2026-11-25d/`.

## Setup

- **Corpus and tasks:** the same httpx and vite checkouts and the same four tasks as pilot 3, with the same grading keys.
- **Runs:** 4 fresh runs per arm per task, 32 runs. Arm A has no graphdown; arm B has the CLI and the updated skill in its prompt.
- **Grading:** two blind graders scored each run 0, 0.5 or 1 from the answer and the diff, and the two scores are averaged. They agreed on 24 of 32. The disagreements were all on vite c2: one grader didn't count the missed "Migration Guide" paragraph against the run.
- **Clean-up:** one run (h413) executed twice; the first result is kept. No arm-A run used graphdown.

The same selection bias applies as in pilot 3: these tasks were kept because an arm-A run failed them.

## Results

| Arm | Mean score | Got the hidden point | Tokens above baseline | Seconds |
| --- | ---------- | -------------------- | --------------------- | ------- |
| A   | 0.69       | 0.94                 | 11.7k                 | 38      |
| B   | 0.75       | 1.00                 | 17.9k                 | 45      |

| Task | A scores | B scores |
| --- | --- | --- |
| httpx c1, timeout default | 0.5 ×4 | 0.5 ×4 |
| vite c2, new future flag | 0.75 ×4 | 0.75 ×4 |
| httpx q2, proxy routing | 1 ×4 | 1 ×4 |
| vite q2, future warnings | 0.5 ×4 | 0.5, 1, 1, 0.5 |

Arm B is ahead by 0.06, all from one task, at about 50% more tokens above baseline. With four runs per cell that is not distinguishable from noise. What the runs show is more useful than the headline:

1. **The mentions feature works when agents call it.** In httpx c1, every arm-B run ran `graphdown refs httpx/_config.py#DEFAULT_TIMEOUT_CONFIG --mention timeout=5`, which lists `docs/logging.md`'s sample output, and all four updated it. Arm A updated it in 3 of 4 this time. Across pilots 2 to 4, arm B has updated `docs/logging.md` in 9 of 9 runs and arm A in 5 of 10.
2. **Agents don't call it when grep already found something.** In vite c2, all 8 runs again missed one of the two places in `docs/changes/ssr-using-modulerunner.md` that needed an update (seven missed the "Migration Guide" paragraph, one missed the warning box). `graphdown refs packages/vite/src/node/server/index.ts#ViteDevServer.ssrFixStacktrace` lists that paragraph as a mention, but no arm-B run asked. They grepped for the flag name, found the warning box, and went straight to editing. The skill step "before changing a definition, run refs" did not survive a task where the agent felt it had already found the doc.
3. **The doc map keeps finding the page keyword search misses.** In vite q2, arm B read every page under `docs/changes` and two of four runs reported that `builder.sharedConfigBuild` has no warning flag. No arm-A run did, in this pilot or the last. Across pilots 3 and 4 that is 4 of 7 arm-B runs and 0 of 7 arm-A runs, the clearest effect seen so far.
4. **Some misses aren't about navigation.** Every httpx c1 run in both arms lost half a point for not adding a test of the new default. Only one arm-B run noticed that the CLI in `httpx/_main.py` has its own copy of the default.

The cost is the reading. Arm B's extra 6k tokens are mostly `outline` and `show` calls; vite's `outline` alone is about 11k characters, and some runs called it twice.

## What this suggests

- **Make the mention check part of the edit, not a separate step.** Agents follow instructions tied to an action they are already doing. Options:
  - `graphdown check` could report unlinked mentions of symbols changed since the last commit, so the check agents already run before finishing (all four vite c2 arm-B runs ran it) surfaces the paragraph.
  - The skill could say "after editing a code symbol, run `refs` on it" rather than "before".

  The first is cheaper for agents and works without the skill. It needs git to know what changed; @docs/design should record whether that is acceptable.
- **Keep the doc map.** Pattern 3 has now held across two pilots and is the effect worth testing at scale.
- **Trim `outline` on big repos.** A second `outline` call is pure waste; the skill should say to keep the first one.
- **Then the larger study**, as before: about 10 runs per cell, with tasks screened by a different arm-A run than the ones measured.
