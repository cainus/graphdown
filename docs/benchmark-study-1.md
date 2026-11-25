# Benchmark study 1: 12 stale-doc tasks, 2026-11-25

The larger study planned in @docs/benchmark, run after the fixes from pilots 3 to 5. Raw data is in `bench/study-2026-11-25/`.

## Setup

- **Corpus:** httpx (Python), vite (TypeScript) and mdBook (Rust), as found, broken links included.
- **Tasks:** 4 change tasks per repo, written by agents told nothing about graphdown (`bench/corpus/writer-prompt-stale.txt`). Each is a small code change that leaves at least three doc passages plainly false, and at least one of those passages is hard to find by searching the task's own words. Every key was checked against the checkouts before any run, and no task was screened on how agents did.
- **Runs:** 5 per arm per task, 120 runs, in shuffled order. Arm A has no graphdown; arm B has the CLI and the skill in its prompt (with `check`'s changed-code list, @docs/cli#changed-code). Each agent wrote its answer to a file, and none ran twice.
- **Grading:** two blind graders per task, scoring 0, 0.5 or 1 from the answer and the diff.

## A grading correction

Six task keys said the changelog "must not be edited". The task writers turned the instruction to keep history out of the keys into a ban. Many agents in both arms added a normal new changelog entry, and the first grading cost them half a point for it. In httpx that was every run of three tasks. The six tasks were graded again with one added rule: a new changelog entry is neutral, and rewriting past entries is still an error (`grading-header-v2.txt`). The other six tasks keep their first grades. Both sets of grades are in the data.

| Grading | Arm A | Arm B | Grader agreement |
| --- | --- | --- | --- |
| First | 0.77 | 0.81 | 110 / 120 |
| Corrected | 0.92 | 0.97 | 119 / 120 |

The rest of this page uses the corrected grades.

## Results

| Arm | Mean score | Full marks | Tokens above baseline | Median seconds |
| --- | --- | --- | --- | --- |
| A | 0.92 | 50 / 60 | 18.5k | 46 |
| B | 0.97 | 56 / 60 | 25.2k | 47 |

| Task | A | B |
| --- | --- | --- |
| httpx c1, default timeout | 1.00 | 0.90 |
| httpx c2, Response repr | 1.00 | 1.00 |
| httpx c3, URL repr | 0.90 | 0.90 |
| httpx c4, drop certifi | 1.00 | 1.00 |
| mdBook c1, default build dir | 0.90 | 1.00 |
| mdBook c2, opt-in extensions | 1.00 | 1.00 |
| mdBook c3, opt-in create-missing | 0.70 | 0.85 |
| mdBook c4, theme folder name | 0.90 | 1.00 |
| vite c1, cache dir | 1.00 | 1.00 |
| vite c2, library formats | 1.00 | 1.00 |
| vite c3, manifest path | 1.00 | 1.00 |
| vite c4, dev port | 0.60 | 1.00 |

Arm B is ahead by 0.05 on average. A permutation test within tasks gives p ≈ 0.05: borderline, from a study this size. Arm B costs about 36% more tokens above the baseline.

## What the numbers mean

- **Most tasks are at the ceiling for both arms.** On 7 of 12 tasks both arms scored 1.00 or the same. Today's agents find a value that's written four different ways across a docs folder by searching hard, and they did, without graphdown.
- **Half the gap is one judgement call.** In vite c4 (dev port 5173 to 5273), three of the four arm-A misses found the create-vite template READMEs and chose not to change them. Their reason was that the templates install the published Vite, which still uses 5173. Every arm-B run changed them. No graphdown output pointed at those files (`refs --mention port=5173` doesn't match `localhost:5173`). The difference is the skill's push to keep every doc consistent, not navigation. Without vite c4, arm B is ahead by 0.02.
- **The rest is small and in mdBook.** mdBook has 391 markdown files, mostly test fixtures, so it is where a map should help most. Arm B was slightly ahead on three of its four tasks, by one run's half point or so each. That is too little to call.
- **Arm B didn't avoid the misses that remain.** Its two non-perfect runs on httpx tasks were an unchanged code comment and leftover dead code, which navigation doesn't help with.
- **The skill was followed.** Every arm-B run ran `outline` and `check`, and 52 of 60 ran `refs`, usually with `--mention` and the old value.

## Conclusions

On tasks where the docs that need changing are plainly wrong after the change, current agents already find nearly all of them by searching. Graphdown didn't measurably improve that, and it costs about a third more tokens. The effects seen in earlier pilots were real but narrow:
- The doc map surfaces a page that keyword search misses (@docs/benchmark-pilot-3#results).
- The changed-code list puts an unlinked mention in front of the agent (@docs/benchmark-pilot-5#results).

This study's tasks contained few such cases. When they did, agents already found them with grep.

So graphdown's case as an aid to correctness on doc-update tasks is weak. What's left to test:

- **Questions instead of changes.** The vite q2 result (`builder.sharedConfigBuild`, 4 of 7 against 0 of 7) came from a question, where nothing in the task names the page to read.
- **Bigger doc sets.** mdBook is the largest corpus here and showed the only consistent lean. A repo with thousands of doc pages would test whether the map matters at scale.
- **Cost.** Arm B's extra tokens are mostly `outline` on large repos. Cutting that cost is worth doing whether or not the accuracy gain holds up.
- **Value outside agent accuracy.** `check` in CI keeps references from breaking, and these benchmarks don't measure that.
