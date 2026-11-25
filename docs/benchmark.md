# Benchmark plan

Graphdown's claim is that agents do better work on documented code when they can address, slice and reverse-search docs, not just read files. Without evidence it is a link checker (@docs/design). This page is the plan for getting that evidence. Two pilots have run: @docs/benchmark-pilot on this repo, and @docs/benchmark-pilot-2 on httpx and vite.

## Question

On tasks that depend on docs, does an agent with graphdown get more of them right, or reach the same results with less reading, than the same agent without it? And is any gain from the CLI, or just from writing docs with explicit references?

## Arms

Every arm gets the same repo, model, task prompt and time limit. Only the following differ.

| Arm | Docs                                          | graphdown CLI | Skill (@skill/SKILL.md) |
| --- | --------------------------------------------- | ------------- | ----------------------- |
| A   | as found: ordinary markdown links             | no            | no                      |
| B   | as found                                      | yes           | yes                     |
| C   | converted with `graphdown fmt --to at`        | no            | no                      |

A against B measures the tool. A against C measures the reference convention on its own. If C is as good as B, the value is in the convention and the CLI is optional; if B is no better than A, graphdown is a link checker and should be pitched as one.

## Corpus

Two or three real open-source repos with substantial markdown docs that describe their code: architecture notes, specs, runbooks. Roughly 30 to 200 doc files each, in Rust, TypeScript or Python so code references resolve. Docs are used as found, plus whatever `graphdown check` needs fixed so that every arm starts from valid references.

Graphdown's own repo is used only for the pilot. Its docs were written with graphdown in mind, so it would flatter arm B.

## Tasks

About 20 tasks per repo, written before any arm is run, by someone (or a separate agent) who has not seen graphdown, from the docs and code alone. Each has an answer key or an automated check.

1. **Multi-hop questions.** The answer needs two or three sections that reference each other, in different files. "What happens to queued writes when a token refresh fails?" Graded against a written answer.
2. **Constrained changes.** A code change where a rule that governs it lives in a doc the task does not mention. "Add a retry to the upload client" when the backoff policy is documented elsewhere. Graded by tests plus a check for the documented rule.
3. **Change and update.** Change a behaviour and keep the docs true. Graded by recall: of the doc sections that describe the changed code (fixed in advance), how many were updated correctly.
4. **Doc restructuring.** Rename a heading, split a doc or move a file. Graded by whether every reference still resolves and still means the same thing.

Types 1 and 2 test reading (`outline`, `show`, `slice`); 3 tests `refs`; 4 tests `check`.

## Measures

- **Correct:** pass or fail per run, from the key or tests. The headline number.
- **Reading cost:** input tokens, tool calls, and distinct files read, from the transcript.
- **Time:** wall clock per run.
- **Collateral damage:** references broken by the run, from `graphdown check` afterwards (run for every arm, after the agent finishes).
- **Doc recall:** task type 3 only, as above.

## Harness

- Each run starts from a fresh worktree of the arm's repo state, with no memory of earlier runs.
- Agent: Claude Code in headless mode, with streamed JSON output kept as the transcript. Repeat the main comparison with one other agent, such as Codex, to check the result is not specific to one harness.
- Five runs per task per arm, because single runs vary too much to compare. For 20 tasks across 3 arms that is 300 runs per repo, so the pilot comes first.
- Grading is blind. The grader sees the final answer or diff and the key, never the transcript or the arm, so graphdown commands can't give the arm away.

## Decision rule

Set before running:

- **Keep building the CLI** if B beats A by at least 10 points of correctness overall, or matches A's correctness with at least 30% fewer input tokens.
- **Pitch the convention, not the tool,** if C captures most of B's gain.
- **Reposition as a link checker** if B is within noise of A on correctness and cost.

## When it doesn't help

A null result is only useful if it says why. Graphdown can fail at four separate points, and the plan measures each one so the fix can be aimed at the right one.

| Where it can fail | How we'd see it | What we'd try |
| --- | --- | --- |
| **Adoption:** the agent never runs graphdown, or gives up on it | Share of arm-B runs that call graphdown at all, and how early | Sharper skill description, an `AGENTS.md` line, a hook that suggests `graphdown show` when an agent opens a doc, an MCP server |
| **Usage:** it runs, but the wrong command or too broadly | Command mix per run, output size of each call, tokens spent on graphdown output against tokens saved elsewhere | Token budgets on `slice` and `outline`, `outline` filtered to a folder, better examples in the skill |
| **Retrieval:** used well, but the output lacks what the task needs | Retrieval recall: whether the task's key sections, listed in advance, ever appeared in the transcript | Denser references (a command that suggests missing ones), `Spec:` back-links, parent-section context in `show` |
| **Reasoning:** the right text was found, the answer is still wrong | Key sections retrieved but task failed | Not a graphdown problem; note it and move on |

Two extra arms, run on a subset of tasks, separate adoption from usefulness:

- **B-forced:** arm B, but the prompt names the graphdown command to start with, for example "begin with `graphdown slice docs/auth#token-refresh`". This is a ceiling: the most the tool can give when used perfectly.
- **B-no-skill:** the CLI is installed but no skill is given, to see whether the skill is what makes agents use it.

Reading them together: if B-forced beats A but B does not, the tool works and agents aren't using it, so fix adoption. If even B-forced doesn't beat A, the output itself isn't good enough, so look at usage and retrieval.

Two properties of the corpus may explain results, so each is recorded per repo: **size** (doc count and total tokens; at small sizes plain grep may already be enough) and **reference density** (references per doc section; docs with few links give `slice` little to follow). Plotting the gain against both shows whether graphdown only pays off past some size or density, which would say who it is for.

Every graphdown call, with its arguments and output size, is logged alongside the transcript so these numbers can be computed after the fact without rerunning anything.

## Pilot

Run 6 tasks (some of each type) on this repo, 2 runs per arm. The goal is to debug the harness, grading and transcript parsing, not to draw conclusions. Then fix the tasks that turn out ambiguous and move to the real corpus.
