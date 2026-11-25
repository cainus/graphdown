---
name: graphdown
description: Navigate and edit markdown docs with the graphdown CLI. Use when a repo has graphdown.toml, when docs or code comments contain @path#section references, or before reading, writing or changing markdown docs that describe code.
---

# Graphdown

Graphdown treats markdown like code: every heading is a symbol, and `@path#fragment` references are checked edges between docs and code. Use the CLI to find exactly the text you need instead of reading whole files, and to keep references valid when you change things.

## Reading docs

Do this instead of listing folders and opening files:

1. `graphdown outline`: one line per doc, with its title and summary. Start here. Add a folder (`graphdown outline docs/guide`) to narrow it.
2. `graphdown outline docs/auth`: one doc's headings, with slugs and line ranges. `--depth 2` keeps only the top levels of a long doc.
3. `graphdown show docs/auth#token-refresh`: print one section, with its subsections.
4. `graphdown slice docs/auth#token-refresh`: that section plus everything it references, transitively, each piece once. Use it to gather context for a task in one read. Add `--depth 1` to stay close, or `--no-code` for prose only.
5. `graphdown outline src/auth.rs` and `graphdown show src/auth.rs#Session::refresh` work on code the same way.

Before changing a function or a doc section, run `graphdown refs <target>` to see every doc and code comment that points at it. Those are what your change may make wrong.

## Writing references

- Docs: `@docs/auth#token-ttl`. The path starts at the repo root (the folder with `graphdown.toml` or `.git`), never at the current file. `.md` may be left off.
- The fragment is the heading's GitHub slug: lowercase, spaces become `-`, punctuation dropped. `## Token TTL` is `#token-ttl`. Copy slugs from `graphdown outline` rather than guessing.
- Code: `@src/auth.rs#Session::refresh` or `@web/client.ts#ApiClient.retry`. Name the definition, never a line number.
- Ordinary markdown links like `[Token TTL](auth.md#token-ttl)` are checked too, relative to the file as usual.
- Anything in backticks or a code block is an example, not a reference. Put examples there, and write npm scopes or handles as `` `@scope/name` ``.
- When a function implements documented behaviour, end its doc comment with `Spec: @docs/<doc>#<section>`. In code comments a reference must contain `/` or `#`.

Do not add `imports:` or `exports:` lists, ids or symbol indexes to frontmatter. Graphdown works all of that out from the headings and references, and a hand-kept copy goes stale.

## Keeping references valid

- Run `graphdown check` after editing docs, renaming or moving a heading, file or function, and before you finish. It prints `file:line:col: reference: reason`, often with a "did you mean". Fix every report.
- Renaming a heading changes its slug. Run `graphdown refs` on the old target first, then update each reference it lists.
- Never rewrite the `@` lines in `CLAUDE.md`: Claude Code reads them as file imports. `graphdown fmt` already skips that file.

`graphdown --help` and `graphdown <command> --help` have the flags. If the binary is missing, run it with `npx @cainus/graphdown` or `uvx graphdown` (same arguments), or install it with `cargo install graphdown`. If none of those work, carry on without it and say so.
