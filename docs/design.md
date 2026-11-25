# Design decisions

Graphdown started from a sketch of "markdown with imports and exports". Most of that sketch was cut. This page records what was cut and why, so the reasons are not rediscovered the hard way.

The rule behind almost every cut: **information that can be derived from the docs is derived, never written down a second time.** A second copy drifts, and an agent trusts a stale index more than it should.

## No import or export lists

The sketch had frontmatter like `exports: [token-refresh]` and `imports: [sync-engine#backoff-policy]`. Both repeat what the document already says. Every heading is a symbol, so the exports are the headings. Every reference in the prose is a dependency, so the imports are the references. @docs/cli#outline and @docs/cli#refs compute both on demand.

An exports list does have one real benefit: an agent opening a file sees its table of contents first. If agents turn out to read files directly rather than running `outline`, the fix is a generated list that `fmt` writes and `check` verifies, not a hand-kept one.

## Paths, not ids

The sketch gave each doc an `id:` separate from its file name, which gives every doc two names. Code imports use paths and so does graphdown: `@docs/syntax` is `docs/syntax.md`. Renaming a doc is a file move plus fixing what `check` reports.

## Root-relative paths, `#` for fragments

`@local-storage/Keychain Store` reads well but cannot be parsed in running prose: nothing marks where it ends, and `/` is already the folder separator. Graphdown uses `/` for folders and `#` for the fragment, the same split as a URL, and fragments are slugs, so they never contain spaces. See @docs/syntax#at-references.

Paths are relative to the project root rather than the current file so that a reference can be copied between docs unchanged. Markdown links keep their usual file-relative meaning, so existing docs keep working.

## Two reference forms

`@` references are what people want to type; markdown links are what GitHub renders as clickable. Both are first-class, and @docs/cli#fmt converts losslessly in either direction.

The `@path` form has precedent beyond chat mentions: Claude Code's `CLAUDE.md` files import other files with `@path/to/file`, and this repo's `CLAUDE.md` pulls in @AGENTS that way. `fmt` leaves `CLAUDE.md` files alone so those imports keep working.

## No committed index

The sketch generated a `SYMBOLS.json` with line numbers at commit time. Line numbers change on nearly every edit, so the file would be stale between commits and a constant source of merge conflicts. A native binary can parse the whole doc set faster than an agent can read that file, so @docs/cli#outline answers the same questions live.

## No compiled copy

The sketch compiled `docs/src` into `docs/dist`. Two copies of every doc means agents read, or worse edit, the wrong one. Graphdown checks docs where they are, and `fmt` rewrites them in place.

## No embeddings

Similarity search is useful for finding where to start, but it cannot follow a dependency whose wording differs from the question, and it adds a model and an index to keep fresh. For doc sets that fit in a few thousand tokens of outline, an agent reading `graphdown outline` routes better than a small embedding model. Revisit if a project's outline outgrows a prompt.

## Back-links from code

A comment like `// Spec: @docs/auth#token-refresh` records an edge that `graphdown refs` could already find from the doc side, so it bends the no-duplication rule above. It is allowed anyway because it puts the pointer where a reader already is: an agent editing a function sees the spec without running anything. Borrowed from lat.md's `@lat:` comments.

Three choices keep the cost down. No marker is required, so there is nothing new to learn. A reference in code must contain `/` or `#`, so documentation tags like `@param` are never misread. And nothing forces the two directions to pair up: each reference is checked on its own, so a missing back-link is never an error. See @docs/syntax#references-from-code.

## Distribution

Graphdown is one static binary, and every channel delivers that same binary rather than a port or a wrapper library. Agent users mostly live in JavaScript and Python repos, so npm and PyPI matter more than crates.io: `npx` and `uvx` let a repo run graphdown without anyone installing Rust. The npm name `graphdown` belongs to an unrelated 2022 package that turns ASCII diagrams into SVG, so the npm package is scoped as `@cainus/graphdown`. The PyPI and crates.io names were free. Releases are cut by tag only, so no channel can drift from the others. See @docs/releasing.

## Open questions

- **`context:` frontmatter.** The one dependency that cannot be derived: a doc that should always come along in a slice even though the prose never mentions it. Add only when a real doc needs it.
- **Coverage.** An opt-in rule that every section under some folder (say `docs/requirements/`) is mentioned by at least one code comment or test. Useful for specs, but it is policy rather than navigation.
- **More languages.** Go, Java and C# are each one tree-sitter grammar and one walker.
