# Graphdown CLI

One binary, `graphdown`, run anywhere inside a project. Targets on the command line use the reference form from @docs/syntax#at-references, with or without the leading `@`: `docs/syntax#sections`, `src/project.rs#Project::resolve`.

`--root DIR` overrides the project root for any command. Files ignored by `.gitignore` or `.graphdownignore` are not loaded as docs.

## check

```sh
graphdown check          # exit 1 if any reference is broken
graphdown check --json
```

Resolves every reference in every doc. A broken one is reported as `file:line:col: reference: reason`, with a "did you mean" when a heading or symbol is close. Put it in CI and in a pre-commit hook; it is the type checker that keeps the rest of the commands honest. Implemented by @src/ops.rs#check.

## outline

```sh
graphdown outline                     # every doc: summary and headings
graphdown outline docs/syntax         # one doc
graphdown outline src/project.rs      # the definitions in a source file
graphdown outline --json
```

Prints each heading (or definition) with its line range and fragment, plus how many references point at the file. Run with no arguments it is the map of the whole doc set, the job a committed `SYMBOLS.json` would otherwise do, but always current. Alias: `ls`.

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

Lists every reference that lands on the target, as `file:line (#section)  reference`. A whole doc or file matches references to any part of it, and a section matches references to its subsections. Run it before changing code or a section, to find the docs that describe it. Implemented by @src/ops.rs#incoming.

## slice

```sh
graphdown slice docs/syntax#references
graphdown slice docs/cli#check docs/design --depth 1 --format xml
graphdown slice docs/architecture --no-code
```

Prints the given targets, then everything they reference, then everything those reference, breadth first. Each section or definition appears once; one already contained in an earlier piece is skipped. The result is the smallest self-contained context for a task, collected in one command instead of a chain of file reads.

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

## Exit codes

| Code | Meaning                                         |
| ---- | ----------------------------------------------- |
| 0    | success                                         |
| 1    | `check` found broken references, or `fmt --check` found changes |
| 2    | usage error, unknown target, unreadable file    |
