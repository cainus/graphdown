# Benchmark pilot 2: httpx and vite, 2026-11-25

The second pilot, run on two real repos after @docs/benchmark-pilot showed that graphdown's own repo was too small and leaked the tool into every arm. Raw data is in `bench/pilot-2026-11-25b/`.

## Setup

- **Corpus:** encode/httpx (26 docs, Python) and vitejs/vite (82 docs, TypeScript), shallow clones as of 2026-11-25. Both use plain markdown links, so arm A is "docs as found" with nothing changed.
- **Tasks:** 4 per repo, written by agents that were told nothing about graphdown, using `bench/corpus/writer-prompt.txt`. The tasks are two multi-hop questions, one change governed by a rule in a doc the task doesn't mention, and one change whose docs must be updated.
- **Arms:** A (no tool) and B (CLI plus skill). Arm C was dropped: `fmt --to at` only converts links whose text equals the generated text, so on a foreign repo C barely differs from A.
- **Runs:** 32, each a fresh Claude Code subagent in its own folder. A run that only loads its context costs 56.7k tokens, so tokens are reported above that baseline.
- **Grading:** two blind graders scored every run 0, 0.5 or 1 against the key, from the final answer and the actual diff. Scripted checks covered which files each change touched and its hidden constraint.

Contamination was low this time. No arm-A run used graphdown, because nothing in a foreign repo points at it.

## Results

Four runs were excluded or discounted. Two code-change runs (s11, s16) executed twice in the same folder and are excluded. Two question runs (s05, s08) also ran twice, which is harmless, so their first answer counts. That leaves 15 valid runs per arm.

| Arm | Mean score | Tokens above baseline | Tool calls | Seconds |
| --- | ---------- | --------------------- | ---------- | ------- |
| A   | 0.93       | 10.1k                 | 5.7        | 30      |
| B   | 0.98       | 12.8k                 | 6.1        | 34      |

The two graders agreed on every run but one, where they differed by half a point.

- **Correctness is near the ceiling again.** 28 of 30 valid runs scored 1 from both graders. The whole gap between arms is one task: httpx u1 (default timeout 5s to 10s). Both arm-B runs updated `docs/logging.md`, whose sample output shows `timeout=5.0`. Both arm-A runs left it, calling it "sample output". With 2 runs per cell that is a hint, not a result. Arm B's calls show `refs` on the timeout constant, which doesn't point at `logging.md`, so the CLI may not even be the reason.
- **Arm B cost more, not less.** It used 27% more tokens above baseline and about 4 seconds longer per run. There was no task where it read less.
- **Adoption is still fine.** Every arm-B run used graphdown, mostly `outline` then `show`, with `check` after changes.
- **The big finding: `outline` doesn't scale.** On vite, `graphdown outline` with no arguments prints about 170,000 characters, every heading in 82 docs. Agents coped by piping it through `head` and `grep`, which throws most of it away. On httpx it is 29k. The plan's "usage" failure (@docs/benchmark#when-it-doesnt-help) is the clearest thing these runs show. The map that is meant to save reading is itself too big to read.

## What to change

1. **Make `outline` usable on big repos.** With no arguments, list one line per doc (path, title and summary) and leave out the headings. Take folders as arguments (`graphdown outline docs/guide`), and add a `--depth` for heading levels. That should cut vite's first call from 170k characters to roughly 8k.
2. **Then rerun this pilot** to see whether arm B's extra cost goes away.
3. **Harder tasks.** A strong model answers these by grep in about four tool calls. Tasks need more hops, or a larger repo where grep returns too much, before a navigation tool can pay off. Pick tasks where an arm-A pass first shows real failures.
4. **Harness:** runs that execute twice must be detected and rerun in a new folder. The real study still needs clean headless agents with transcripts, which would also show whether arm B's `logging.md` edit came from graphdown at all.
