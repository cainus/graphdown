# Benchmark pilot, 2026-11-25

The pilot from @docs/benchmark#pilot: 6 tasks on this repo, 3 arms, 2 runs each, 36 runs. As planned, it says nothing about whether graphdown helps. What it does say is how to fix the harness before the real study. Raw data is in `bench/pilot-2026-11-25/`.

## How it ran

Headless `claude -p` could not sign in from the cloud workspace the pilot ran in, so each run was a fresh Claude Code subagent given only a pointer to its prompt file. Each run had its own copy of the repo, built by `bench/setup.sh`, under a neutral name. Change tasks were graded by `bench/grade.sh`. The question answers, and the summaries for task t4, were graded blind by a separate agent against the keys, with the run IDs shuffled. One run (r34) is excluded, for the reason given under harness problems, so the numbers below cover 35 runs.

## Results

Every valid run succeeded on every main check. All builds and test suites passed, every reference resolved afterwards, and every answer matched its key. The blind grader marked one correct answer wrong, and its own note contradicted the mark, so it was counted correct.

| Arm | Runs | Mean tokens | Mean tool calls | Mean seconds |
| --- | ---- | ----------- | --------------- | ------------ |
| A   | 11   | 65.7k       | 7.8             | 71           |
| B   | 12   | 67.2k       | 7.8             | 77           |
| C   | 12   | 65.8k       | 6.8             | 78           |

There is no difference worth reading, for three reasons: the arms were contaminated, the tasks were too easy, and the token count measures the wrong thing. Each is covered below.

What did come out of it:

- **Adoption is not the problem when the skill is present.** Arm B called graphdown in 12 of 12 runs, always early, usually `outline` first and `refs` before editing. Output stayed small: 1.9k to 11.8k characters per run.
- **Every arm missed the same doc.** In t4 (`&` becomes `and` in slugs), all 6 runs updated `docs/syntax.md` and none updated `skill/SKILL.md`, which restates the slug rule in its own words. Arm B ran `refs` on the rule's section and still couldn't see it, because the skill restates the rule without referencing it. Prose that duplicates another section without linking to it is invisible to graphdown. This is the "retrieval" failure from @docs/benchmark#when-it-doesnt-help, and the first real product finding.
- **All 6 t4 runs flagged the hidden conflict:** the change breaks the docs' promise that slugs match GitHub's anchors. All three arms found it, so this task can't tell them apart.

## Harness problems

1. **The arms were not isolated.** 15 of 23 runs in arms A and C used graphdown anyway, through `cargo run`, because they could still find it. Subagents inherit the session's project instructions: four runs quoted an `AGENTS.md` that does not exist in their copy. One run loaded the graphdown skill saved to the user's account, and one read the user's memory about this project. The real study must run agents in a clean environment with no account skills, no memory, and no parent project instructions: headless agents with their own API key, in a container.
2. **Tokens measured the wrong thing.** About 55k of every run's tokens was the same fixed context, so differences in reading were lost in it. Record per-call input and output from transcripts, and report tokens above each run's starting context.
3. **The tasks were too easy and the corpus too small.** Ten docs are easy to grep, and every arm hit the ceiling. Use the real corpus from @docs/benchmark#corpus, and add tasks whose key sections are several hops apart.
4. **An interrupted run kept going.** When a run was cancelled and restarted, the first attempt carried on in the same folder, so r34 had two agents editing at once and is excluded. Give every attempt a fresh folder, and never reuse one.
5. **Blind grading needs a second grader.** One false mark in 18 is too many for a headline number. Grade twice, and resolve disagreements by hand or with a stricter key.
6. **Tool counts undercount.** One tool call often ran several shell commands, so counting calls says little about how much was read. Count files and bytes read from transcripts instead.

## Next

- Fix the harness problems above and rerun the pilot on the real corpus.
- In graphdown itself, act on the missed-doc finding. Either `check` warns when two sections state the same rule without referencing each other, or the skill tells agents to `grep` for a rule's key terms as well as running `refs`. The first is a feature to design; the second is one line of the skill.
