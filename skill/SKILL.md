---
name: graphdown
description: Keep markdown docs and their references to code valid with the graphdown linter. Use when a repo has graphdown.toml, when docs or code comments contain @path#section references, or before finishing a change that touches docs or documented code.
---

# Graphdown

Graphdown is a linter for docs. It checks that every markdown link and every `@path#fragment` reference lands on a real file, heading or code definition, and it points out docs that name code you just changed.

## Before you finish

- Run `graphdown check`. It prints `file:line:col: reference: reason`, often with a "did you mean". Fix every report.
- In a git repo, `check` also lists doc lines that reference or name code you changed but that you didn't edit. Open each one and either update it or decide it still holds. They don't fail the check, so it is easy to skip them; don't.
- Before renaming a heading, file or function, run `graphdown refs <target>` to list everything that points at it, plus doc lines that name it without a link. Update each one.

## Writing references

- Docs: `@docs/auth#token-ttl`. The path starts at the repo root (the folder with `graphdown.toml` or `.git`), never at the current file. `.md` may be left off.
- The fragment is the heading's GitHub slug: lowercase, spaces become `-`, punctuation dropped. `## Token TTL` is `#token-ttl`.
- Code: `@src/auth.rs#Session::refresh` or `@web/client.ts#ApiClient.retry`. Name the definition, never a line number.
- Ordinary markdown links like `[Token TTL](auth.md#token-ttl)` are checked too, relative to the file as usual.
- Anything in backticks or a code block is an example, not a reference. Write npm scopes or handles as `` `@scope/name` ``.
- When a function implements documented behaviour, end its doc comment with `Spec: @docs/<doc>#<section>`. In code comments a reference must contain `/` or `#`.
- Never rewrite the `@` lines in `CLAUDE.md`: Claude Code reads them as file imports. `graphdown fmt` already skips that file.

`graphdown --help` has the flags. If the binary is missing, run it with `npx @cainus/graphdown` or `uvx graphdown` (same arguments), or install it with `cargo install graphdown`. If none of those work, carry on without it and say so.
