# Graphdown CLI

One binary, `graphdown`, run anywhere inside a project. Targets on the command line use the reference form from @docs/syntax#at-references, with or without the leading `@`: `docs/syntax#sections`, `src/project.rs#Project::resolve`.

`--root DIR` overrides the project root for any command. Files ignored by `.gitignore` or `.graphdownignore` are not loaded as docs.

## check

```sh
graphdown check                  # exit 1 if any reference is broken
graphdown check --json           # same as --format json
graphdown check --format github  # GitHub Actions annotations
graphdown check --since main     # compare against main, not HEAD
graphdown check --no-changes     # skip the changed-code list
```

Resolves every reference in every doc and in every code comment (@docs/syntax#references-from-code). A broken one is reported as `file:line:col: reference: reason`, with a "did you mean" when a heading or symbol is close. Put it in CI and in a pre-commit hook; it is the type checker that keeps the rest of the commands honest. With `--format github`, each problem is also printed as a workflow command, so GitHub shows it as an error on that line of the pull request; the repo's own Action uses this (@docs/releasing#the-action). Implemented by @src/ops.rs#check.

### Changed code

In a git repository, `check` also lists the lines that name code you have changed but that the change itself didn't touch: the docs most likely to now describe the old behaviour. These are notes, not errors, and never change the exit code.

It compares the working tree, staged or not, with `--since` (default `HEAD`), using `git diff` (@src/changes.rs#since). Untracked files are left out. For each changed range in a source file it takes the innermost symbol around it, and lists:

- every reference to that symbol from a line outside the change, and
- every doc line outside the change that contains its name as a whole word, the same search as `refs` mentions (@docs/cli#refs).

The name is the symbol's last part, or the nearest named method or property around the edit when that isn't a symbol itself: editing `ssrFixStacktrace() {...}` inside an object literal in `createServer` searches for `ssrFixStacktrace`. Names that read like English words (`main`, `refresh`, anything without a capital after the first letter, an underscore or a digit) are not searched for, because they match ordinary prose.

Each line prints as `file:line (#section)  references|mentions target  text`. With `--format github` they become `::notice` annotations; in `--json` output the report is `{"problems": [...], "stale": [...]}`. Leave changelogs and other history out with `.graphdownignore` if they get in the way. Implemented by @src/ops.rs#stale. In CI, `HEAD` is the commit being checked, so pass the base branch: `--since origin/main`.

## outline

```sh
graphdown outline                         # every doc, one line each: title, summary, size
graphdown outline docs/guide              # the docs in one folder
graphdown outline docs/syntax             # one doc's headings, with line ranges and fragments
graphdown outline docs/syntax --depth 2   # only # and ## headings
graphdown outline src/project.rs          # the definitions in a source file
graphdown outline --full                  # headings for every doc
graphdown outline --json
```

With no arguments, or a folder, it prints one line per doc: its reference, title, summary, length and how many references point at it. A doc or code file lists every heading or definition with its line range and fragment. That two-step map does the job a committed `SYMBOLS.json` would otherwise do, but always current, and stays small on big repos: vite's 82 docs fit in about 11,000 characters as one line each, against 166,000 with every heading (@docs/benchmark-pilot-2#results). Alias: `ls`.

## show

```sh
graphdown show docs/syntax#sections
graphdown show src/project.rs#Project::resolve
```

Prints exactly one section or definition, preceded by a `<!-- target file Lstart-end -->` line so the reader knows where it came from.

## refs

```sh
graphdown refs docs/syntax#sections
graphdown refs src/project.rs         # every doc that mentions the file
graphdown refs src/project.rs#Project::resolve
```

Lists every reference that lands on the target, as `file:line (#within)  reference`, where `within` is the section slug for a doc or the enclosing definition for code. References from code comments are included, so `graphdown refs docs/auth#token-refresh` lists the functions that claim to implement it. A whole doc or file matches references to any part of it, and a section matches references to its subsections. Run it before changing code or a section, to find the docs that describe it.

References are only half the story: docs often name a function or setting in passing without linking it, and those lines go stale silently. So `refs` also lists, separately, every doc line that contains the target's name as a whole word but has no reference to it. For a code symbol the name is its last part (`ssrFixStacktrace` for `ViteDevServer.ssrFixStacktrace`, at least four characters long). Add more words with `--mention`, for example `--mention timeout=5.0` before changing a default, and turn the list off with `--no-mentions`. Mentions are leads to check by hand, not proof: graphdown can't tell whether the word means the target. In `--json` output every entry has a `kind` of `reference` or `mention`.

```sh
graphdown refs src/server.ts#ViteDevServer.ssrFixStacktrace
graphdown refs docs/advanced/timeouts --mention timeout=5.0
```

In @docs/benchmark-pilot-3#results every agent missed a migration paragraph that named `server.ssrFixStacktrace` without linking it; this list puts that paragraph first. Implemented by @src/ops.rs#incoming.

## slice

```sh
graphdown slice docs/syntax#references
graphdown slice docs/cli#check docs/design --depth 1 --format xml
graphdown slice docs/architecture --no-code
```

Prints the given targets, then everything they reference, then everything those reference, breadth first. A code target's comments count as references too, so `graphdown slice src/auth.rs#Session::refresh` brings in the spec its doc comment names. Each section or definition appears once; one already contained in an earlier piece is skipped. The result is the smallest self-contained context for a task, collected in one command instead of a chain of file reads.

- `--depth N` stops after N hops.
- `--no-code` follows doc references only.
- `--format markdown` (default) prints pieces with a header comment, code in fences; `xml` wraps each piece in `<doc>` or `<code>` with `ref`, `file` and `lines` attributes; `json` gives the raw pieces.

Implemented by @src/ops.rs#slice.

## fmt

```sh
graphdown fmt --to links          # @refs become clickable markdown links
graphdown fmt --to at             # and back
graphdown fmt --to links --check  # exit 1 if anything would change
```

Rewrites references in place. Only references that resolve are touched. A link's text comes from the target: the heading text for a section, the title for a doc, the name in backticks for code. Going back, a link is turned into an `@` reference only when its text is exactly what `--to links` would have written, so hand-written link text is never thrown away. References come back in canonical form, so `@AGENTS.md` returns as `@AGENTS`.

`CLAUDE.md` and `CLAUDE.local.md` are never rewritten, because Claude Code reads `@path` in them as a file import. Implemented by @src/ops.rs#format_doc.

## skill

```sh
graphdown skill             # print the agent skill
graphdown skill --install   # write .claude/skills/graphdown/SKILL.md under the project root
```

Prints a short agent skill: when to reach for graphdown, how to read docs with it, how to write references, and how to keep them valid. Its source is @skill/SKILL.md, compiled into the binary, so the instructions always match the installed version. Works outside a project too. For agents that read `AGENTS.md` rather than skills, paste the output there.

## Exit codes

| Code | Meaning                                         |
| ---- | ----------------------------------------------- |
| 0    | success                                         |
| 1    | `check` found broken references, or `fmt --check` found changes |
| 2    | usage error, unknown target, unreadable file    |
