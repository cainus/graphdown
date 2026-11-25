# Benchmark pilot 3: hard tasks, 2026-11-25

The third pilot, after @docs/benchmark-pilot-2 found the tasks too easy and `outline` too big. This one uses the bounded `outline` (@docs/cli#outline) and tasks chosen because agents without graphdown get them wrong. Raw data is in `bench/pilot-2026-11-25c/`.

## Setup

- **Corpus:** httpx and vite, the same checkouts as pilot 2.
- **Tasks:** 12 hard tasks (6 per repo), written by agents told nothing about graphdown but asked for things a hasty agent would miss. They were asked for facts spread over three or more files, stale docs that contradict the code, changes that must reach indirect docs, and rules kept in contributor docs. The prompt is `bench/pilot-2026-11-25c/writer-prompt.txt`.
- **Screening:** one arm-A run per task, graded blind. Eight of 12 were fully solved without graphdown. Four scored half, and those four were kept:
  - httpx c1: raise the default timeout
  - vite c2: add a future-deprecation flag
  - httpx q2: proxy routing from environment variables
  - vite q2: every future-deprecation warning
- **Comparison:** the four tasks, 3 fresh runs per arm, 24 runs. Two blind graders scored each run 0, 0.5 or 1 from the answer and the diff. They agreed on 22 of 24.

Choosing tasks because arm A failed them favours arm B: some of those failures are luck, and fresh arm-A runs will regress towards doing better. The fresh runs limit that, and in practice arm A failed these tasks again at much the same rate.

No run executed twice this time, and no arm-A run used graphdown.

## Results

| Arm | Mean score | Got the hidden point | Tokens above baseline | Seconds |
| --- | ---------- | -------------------- | --------------------- | ------- |
| A   | 0.67       | 0.67                 | 14.9k                 | 45      |
| B   | 0.67       | 0.75                 | 18.6k                 | 48      |

| Task | A scores | B scores |
| --- | --- | --- |
| httpx c1, timeout default | 0.5, 0.5, 1 | 0.5, 0.5, 0.5 |
| vite c2, new future flag | 0.5, 0.5, 0.5 | 0.5, 0.5, 0.5 |
| httpx q2, proxy routing | 1, 1, 1 | 1, 1, 1 |
| vite q2, future warnings | 0.5, 0.5, 0.5 | 0.5, 0.75, 0.75 |

The headline is a tie, at about 25% more cost for arm B. Three patterns sit under it, all from small numbers:

1. **Graphdown helps when the job is "find every doc that might be affected".** In vite q2, arm B listed `docs/changes` with `outline` and then read every page in it, and two of three runs found the one change with no warning flag (`builder.sharedConfigBuild`). Arm-A runs searched for the flag names and never saw that page. The same thing happened across pilots for `docs/logging.md` in the timeout task: its sample output shows `timeout=5.0` without using the words "default" or "timeout setting". Arm B updated it in 5 of 5 runs over pilots 2 and 3; arm A in 2 of 6, counting the screening run. The doc map puts docs in front of the agent that keyword search doesn't find.
2. **Both arms fail at prose that restates a rule without referencing it.** In vite c2, all 6 runs missed the "Migration Guide" paragraph of `docs/changes/ssr-using-modulerunner.md`, which names `server.ssrFixStacktrace` in plain text. `refs` finds references, not mentions, so graphdown is blind to it. This is the same failure as `skill/SKILL.md` in @docs/benchmark-pilot#results.
3. **Thoroughness costs tokens.** Arm B's extra 3.7k tokens per run are mostly the `show` calls that produce pattern 1. In httpx q2, where grep alone was enough, arm B spent 30% more for the same perfect score.

Grading also shows the keys carry some weight: arm A's one perfect httpx c1 run added a test for the new default, which the key asks for. No arm-B run did.

## What this suggests

- **Unlinked mentions.** Make `graphdown refs` also list sections that mention a symbol's bare name (`ssrFixStacktrace`, `DEFAULT_TIMEOUT_CONFIG`) in prose or code spans without referencing it, marked as mentions rather than references. That targets the one failure every arm shared, in two pilots out of three.
- **Point agents at the map before finishing.** The skill could say: before finishing a change, run `graphdown outline` on the folders the change touches and open any doc whose summary relates to it. That is pattern 1 made deliberate, rather than a side effect of starting with `outline`.
- **Then a larger study.** Three runs per cell can't separate a 0.08 gap from noise. The real study needs the clean harness and about 10 runs per cell on tasks like these.
